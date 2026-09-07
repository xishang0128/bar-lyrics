use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use tungstenite::{Message, connect};

const WS_RECONNECT_DELAY: Duration = Duration::from_secs(1);
const WS_CONNECT_ATTEMPTS: usize = 5;

enum UpdateEvent {
    Connected,
    Changed,
    Disconnected(String),
}

pub(super) struct UpdateWatcher {
    receiver: Receiver<UpdateEvent>,
    connected: bool,
    last_error: String,
}

impl UpdateWatcher {
    pub(super) fn connect(base_url: &str, events: Receiver<()>) -> Result<Self, String> {
        let url = websocket_url(base_url)?;
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("splayer-websocket".to_owned())
            .spawn({
                let url = url.clone();
                move || watch_updates(&url, sender, events)
            })
            .map_err(|error| format!("start SPlayer WebSocket watcher: {error}"))?;

        Ok(Self {
            receiver,
            connected: false,
            last_error:
                "waiting for SPlayer WebSocket; enable WebSocket in SPlayer external API settings"
                    .to_owned(),
        })
    }

    pub(super) fn take(&mut self) -> Result<bool, String> {
        let mut changed = false;
        loop {
            match self.receiver.try_recv() {
                Ok(UpdateEvent::Connected) => {
                    self.connected = true;
                    self.last_error.clear();
                    changed = true;
                }
                Ok(UpdateEvent::Changed) => changed = true,
                Ok(UpdateEvent::Disconnected(error)) => {
                    self.connected = false;
                    self.last_error = error;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.connected = false;
                    self.last_error = "watcher stopped unexpectedly".to_owned();
                    break;
                }
            }
        }

        if self.connected {
            Ok(changed)
        } else {
            Err(format!(
                "SPlayer WebSocket disconnected: {}",
                self.last_error
            ))
        }
    }
}

pub(super) fn websocket_url(base_url: &str) -> Result<String, String> {
    let base_url = base_url.trim_end_matches('/');
    if let Some(endpoint) = base_url.strip_prefix("http://") {
        return Ok(format!("ws://{endpoint}/ws"));
    }
    if let Some(endpoint) = base_url.strip_prefix("https://") {
        return Ok(format!("wss://{endpoint}/ws"));
    }
    Err("SPlayer endpoint must start with http:// or https://".to_owned())
}

fn watch_updates(url: &str, sender: Sender<UpdateEvent>, events: Receiver<()>) {
    let mut attempts = WS_CONNECT_ATTEMPTS;
    loop {
        let disconnected = match connect(url) {
            Ok((mut socket, _)) => {
                attempts = WS_CONNECT_ATTEMPTS;
                if sender.send(UpdateEvent::Connected).is_err() {
                    return;
                }
                loop {
                    match socket.read() {
                        Ok(Message::Text(text)) if is_update(text.as_ref()) => {
                            if sender.send(UpdateEvent::Changed).is_err() {
                                return;
                            }
                        }
                        Ok(Message::Close(_)) => break "connection closed".to_owned(),
                        Ok(_) => {}
                        Err(error) => break error.to_string(),
                    }
                }
            }
            Err(error) => {
                attempts -= 1;
                error.to_string()
            }
        };

        if sender
            .send(UpdateEvent::Disconnected(disconnected))
            .is_err()
        {
            return;
        }
        if attempts == 0 {
            if events.recv().is_err() {
                eprintln!("bar-lyrics: reconnect attempts exhausted; restart the plugin to retry");
                return;
            }
            attempts = WS_CONNECT_ATTEMPTS;
        } else {
            // Discard events already covered by this connection attempt.
            while events.try_recv().is_ok() {}
            thread::sleep(WS_RECONNECT_DELAY);
        }
    }
}

fn is_update(message: &str) -> bool {
    #[derive(Deserialize)]
    struct Envelope {
        kind: String,
    }

    serde_json::from_str::<Envelope>(message).is_ok_and(|message| message.kind == "event")
}
