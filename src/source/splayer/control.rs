use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use tungstenite::Message;

use super::{SPlayer, websocket::websocket_url};
use crate::options::Control;
use crate::source::Source;

pub(crate) fn control(endpoint: &str, command: &mut Control) -> Result<(), String> {
    let timeout = Duration::from_secs(3);
    if matches!(command, Control::Toggle) {
        let mut source = SPlayer::new(endpoint, None)?;
        source.agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .build()
            .into();
        *command = if source.now_playing()?.playing {
            Control::Pause
        } else {
            Control::Play
        };
    }
    let op = match *command {
        Control::Play => "play",
        Control::Pause => "pause",
        Control::Previous => "prev",
        Control::Next => "next",
        Control::Toggle => unreachable!(),
    };
    let url = websocket_url(endpoint)?;
    let uri = url
        .parse::<tungstenite::http::Uri>()
        .map_err(|error| error.to_string())?;
    let host = uri
        .host()
        .ok_or("SPlayer endpoint needs a host")?
        .trim_matches(['[', ']']);
    let port = uri
        .port_u16()
        .unwrap_or(if uri.scheme_str() == Some("wss") {
            443
        } else {
            80
        });
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| error.to_string())?;
    let stream = addresses
        .filter_map(|address| TcpStream::connect_timeout(&address, timeout).ok())
        .next()
        .ok_or("SPlayer control connection failed")?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    let (mut socket, _) = tungstenite::client_tls(url, stream)
        .map_err(|error| format!("SPlayer control connection: {error}"))?;
    socket
        .send(Message::Text(
            serde_json::json!({ "op": op }).to_string().into(),
        ))
        .map_err(|error| format!("SPlayer control send: {error}"))?;
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            return Err("SPlayer control acknowledgement timed out".to_owned());
        }
        match socket
            .read()
            .map_err(|error| format!("SPlayer control: {error}"))?
        {
            Message::Text(text) => {
                let reply: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|error| format!("SPlayer control reply: {error}"))?;
                if reply["op"] != op {
                    continue;
                }
                match reply["kind"].as_str() {
                    Some("ack") => {
                        let _ = socket.close(None);
                        return Ok(());
                    }
                    Some("error") => return Err(format!("SPlayer control: {}", reply["error"])),
                    _ => {}
                }
            }
            Message::Close(_) => {
                return Err("SPlayer closed before acknowledging control".to_owned());
            }
            _ => {}
        }
    }
}
