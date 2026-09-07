use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use serde_json::{Value, json};
use tiny_http::{Header, Method, Response, Server};

pub(super) struct Transport {
    pub(super) address: Value,
    server: Arc<Server>,
    discovery: Option<PathBuf>,
}

impl Transport {
    pub(super) fn start(
        mut handle: impl FnMut(&[u8]) -> Option<Value> + Send + 'static,
    ) -> Result<Self, String> {
        let server = Arc::new(Server::http("127.0.0.1:0").map_err(|error| error.to_string())?);
        let host = server.server_addr().to_string();
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).map_err(|error| error.to_string())?;
        let token: String = secret.iter().map(|byte| format!("{byte:02x}")).collect();
        let address = json!({"version": 1, "pid": std::process::id(), "url": format!("http://{host}/rpc"), "token": token});
        let discovery = discover(&address).map_err(|error| error.to_string())?;
        let transport = Self {
            address,
            server: server.clone(),
            discovery,
        };
        thread::Builder::new()
            .name("bar-lyrics-ipc".to_owned())
            .spawn(move || {
                let authorization = format!("Bearer {token}");
                while let Ok(mut request) = server.recv() {
                    let header = |name: &str| {
                        request
                            .headers()
                            .iter()
                            .find(|header| {
                                header.field.as_str().as_str().eq_ignore_ascii_case(name)
                            })
                            .map(|header| header.value.as_str())
                    };
                    let rejection = if header("Authorization") != Some(authorization.as_str())
                        || header("Host") != Some(host.as_str())
                    {
                        Some(403)
                    } else if request.method() != &Method::Post
                        || request.url() != "/rpc"
                        || header("Origin").is_some()
                    {
                        Some(400)
                    } else if !header("Content-Type").is_some_and(|value| {
                        value
                            .split(';')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .eq_ignore_ascii_case("application/json")
                    }) {
                        Some(415)
                    } else if request
                        .body_length()
                        .is_none_or(|size| size > super::MAX_MESSAGE)
                    {
                        Some(413)
                    } else {
                        None
                    };
                    if let Some(status) = rejection {
                        let _ = request.respond(Response::empty(status));
                        continue;
                    }
                    let mut body = Vec::new();
                    if request
                        .as_reader()
                        .take(super::MAX_MESSAGE as u64 + 1)
                        .read_to_end(&mut body)
                        .is_err()
                        || body.len() > super::MAX_MESSAGE
                    {
                        let _ = request.respond(Response::empty(413));
                        continue;
                    }
                    if let Some(reply) = handle(&body) {
                        let response = Response::from_string(reply.to_string())
                            .with_header(
                                Header::from_bytes("Content-Type", "application/json").unwrap(),
                            )
                            .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap());
                        let _ = request.respond(response);
                    } else {
                        let _ = request.respond(Response::empty(204));
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(transport)
    }
}

fn discover(address: &Value) -> std::io::Result<Option<PathBuf>> {
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return Ok(None);
    };
    let directory = PathBuf::from(runtime).join("bar-lyrics");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        match fs::DirBuilder::new().mode(0o700).create(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        let metadata = fs::symlink_metadata(&directory)?;
        if !metadata.is_dir() || metadata.uid() != unsafe { libc::geteuid() } {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "unsafe IPC runtime directory",
            ));
        }
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    fs::create_dir_all(&directory)?;
    let token = address["token"].as_str().unwrap();
    let path = directory.join(format!("{}-{}.json", std::process::id(), &token[..8]));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(&path)?
        .write_all(address.to_string().as_bytes())?;
    eprintln!("bar-lyrics: IPC discovery {}", path.display());
    Ok(Some(path))
}

impl Drop for Transport {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(path) = &self.discovery {
            let _ = fs::remove_file(path);
        }
    }
}
