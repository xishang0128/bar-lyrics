mod transport;

use std::sync::mpsc::{self, Receiver, SyncSender};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::options::{Control, Options};

pub(crate) const MAX_MESSAGE: usize = 64 * 1024;

pub(crate) struct Configure {
    pub(crate) options: Options,
    pub(crate) reply: SyncSender<Result<(), String>>,
}

pub(crate) struct Ipc {
    pub(crate) requests: Receiver<Configure>,
    pub(crate) address: Value,
    _transport: transport::Transport,
}

impl Ipc {
    pub(crate) fn start(options: Options) -> Result<Self, String> {
        let (sender, requests) = mpsc::sync_channel(16);
        let mut protocol = Protocol { options, sender };
        let transport = transport::Transport::start(move |body| protocol.handle(body))?;
        Ok(Self {
            requests,
            address: transport.address.clone(),
            _transport: transport,
        })
    }
}

// The first stdin message is optional; Waybar uses a notification so stdout stays module JSON.
pub(crate) fn bootstrap() -> Result<Option<Options>, String> {
    use std::io::{BufRead, IsTerminal, Read};
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    stdin
        .lock()
        .take(MAX_MESSAGE as u64 + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > MAX_MESSAGE {
        return Err("IPC message too large".to_owned());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if value["jsonrpc"] != "2.0" || value["method"] != "config.update" || value.get("id").is_some()
    {
        return Err("stdin expects a config.update JSON-RPC notification".to_owned());
    }
    Options::default()
        .updated(value["params"].clone())
        .map(Some)
}

struct Protocol {
    options: Options,
    sender: SyncSender<Configure>,
}

impl Protocol {
    fn handle(&mut self, body: &[u8]) -> Option<Value> {
        match serde_json::from_slice::<Value>(body) {
            Err(_) => Some(error(Value::Null, -32700, "Parse error")),
            Ok(Value::Array(batch)) => {
                if batch.is_empty() || batch.len() > 64 {
                    return Some(error(Value::Null, -32600, "Invalid batch"));
                }
                let replies: Vec<_> = batch
                    .into_iter()
                    .filter_map(|request| self.call(request))
                    .collect();
                (!replies.is_empty()).then_some(Value::Array(replies))
            }
            Ok(request) => self.call(request),
        }
    }

    fn call(&mut self, request: Value) -> Option<Value> {
        let id = request.get("id");
        if !request.is_object()
            || request["jsonrpc"] != "2.0"
            || !request["method"].is_string()
            || id.is_some_and(|id| !(id.is_null() || id.is_string() || id.is_number()))
            || request
                .get("params")
                .is_some_and(|params| !(params.is_object() || params.is_array()))
        {
            return Some(error(Value::Null, -32600, "Invalid Request"));
        }
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let result = self.dispatch(request["method"].as_str().unwrap(), params);
        id.map(|id| match result {
            Ok(value) => json!({"jsonrpc": "2.0", "id": id, "result": value}),
            Err((code, message)) => error(id.clone(), code, &message),
        })
    }

    fn dispatch(&mut self, method: &str, params: Value) -> Result<Value, (i32, String)> {
        match method {
            "config.get" if params == json!({}) || params == json!([]) => {
                Ok(serde_json::to_value(&self.options).unwrap())
            }
            "config.get" => Err((-32602, "config.get takes no parameters".to_owned())),
            "config.update" => {
                let options = self
                    .options
                    .updated(params)
                    .map_err(|error| (-32602, error))?;
                if options.output != self.options.output {
                    return Err((-32602, "output is fixed for this stream".to_owned()));
                }
                let (reply, received) = mpsc::sync_channel(1);
                self.sender
                    .send(Configure {
                        options: options.clone(),
                        reply,
                    })
                    .map_err(|_| (-32603, "runtime stopped".to_owned()))?;
                received
                    .recv()
                    .map_err(|_| (-32603, "runtime stopped".to_owned()))?
                    .map_err(|error| (-32603, error))?;
                self.options = options;
                Ok(serde_json::to_value(&self.options).unwrap())
            }
            "player.control" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Params {
                    action: Control,
                }
                let params: Params =
                    serde_json::from_value(params).map_err(|error| (-32602, error.to_string()))?;
                crate::source::control(
                    self.options.source,
                    &self.options.source_endpoint,
                    params.action,
                )
                .map_err(|error| (-32001, error))?;
                Ok(Value::Null)
            }
            _ => Err((-32601, "Method not found".to_owned())),
        }
    }
}

fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}
