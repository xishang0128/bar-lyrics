use std::io::{self, Write};

use crate::engine::Frame;

pub(crate) fn notify(method: &str, params: impl serde::Serialize) -> Result<(), String> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({"jsonrpc": "2.0", "method": method, "params": params}),
    )
    .map_err(|error| error.to_string())?;
    stdout
        .write_all(b"\n")
        .and_then(|()| stdout.flush())
        .map_err(|error| error.to_string())
}

pub(super) struct Ipc;

impl super::Output for Ipc {
    fn emit(&mut self, frame: &Frame) -> Result<(), String> {
        notify("frame", frame)
    }
}
