use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

struct CoverResult {
    url: String,
    path: String,
}

pub(crate) struct CoverCache {
    directory: Option<PathBuf>,
    request_tx: Option<Sender<String>>,
    result_rx: Option<Receiver<CoverResult>>,
    requested_url: String,
    path: String,
}

impl CoverCache {
    pub(crate) fn new(directory: Option<PathBuf>) -> Self {
        let (request_tx, result_rx) = if let Some(worker_dir) = directory.clone() {
            let (request_tx, request_rx) = mpsc::channel();
            let (result_tx, result_rx) = mpsc::channel();
            thread::spawn(move || download_worker(worker_dir, request_rx, result_tx));
            (Some(request_tx), Some(result_rx))
        } else {
            (None, None)
        };
        Self {
            directory,
            request_tx,
            result_rx,
            requested_url: String::new(),
            path: String::new(),
        }
    }

    pub(crate) fn set_url(&mut self, url: &str) {
        self.poll();
        if url == self.requested_url {
            return;
        }
        self.requested_url = url.to_owned();
        self.path.clear();
        if url.is_empty() || self.directory.is_none() {
            return;
        }
        if url.starts_with("file://") || Path::new(url).is_absolute() {
            if let Some(path) = local_path(url) {
                self.path = path;
            }
            return;
        }

        let Some(directory) = &self.directory else {
            return;
        };
        let destination = cache_path(directory, url);
        if destination.is_file() {
            self.path = destination.to_string_lossy().into_owned();
        } else if let Some(request_tx) = &self.request_tx {
            let _ = request_tx.send(url.to_owned());
        }
    }

    pub(crate) fn path(&mut self) -> &str {
        self.poll();
        &self.path
    }

    fn poll(&mut self) {
        let Some(result_rx) = &self.result_rx else {
            return;
        };
        while let Ok(result) = result_rx.try_recv() {
            if result.url == self.requested_url {
                self.path = result.path;
            }
        }
    }
}

fn download_worker(
    directory: PathBuf,
    request_rx: Receiver<String>,
    result_tx: Sender<CoverResult>,
) {
    let agent = ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .new_agent();
    for url in request_rx {
        let path = download(&agent, &directory, &url).unwrap_or_else(|error| {
            eprintln!("bar-lyrics: cover: {error}");
            String::new()
        });
        if result_tx.send(CoverResult { url, path }).is_err() {
            break;
        }
    }
}

fn download(agent: &ureq::Agent, directory: &Path, url: &str) -> Result<String, String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let destination = cache_path(directory, url);
    if destination.is_file() {
        return Ok(destination.to_string_lossy().into_owned());
    }

    let normalized_url = normalize_url(url);
    let mut response = agent
        .get(normalized_url.as_ref())
        .call()
        .map_err(|error| format!("GET {url}: {error}"))?;
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|error| format!("read {url}: {error}"))?;
    if bytes.is_empty() {
        return Err(format!("empty response from {url}"));
    }

    let temporary = directory.join(format!(".cover-{:016x}.part", hash_url(url)));
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if let Err(error) = fs::rename(&temporary, &destination) {
        let _ = fs::remove_file(&temporary);
        if !destination.is_file() {
            return Err(error.to_string());
        }
    }
    Ok(destination.to_string_lossy().into_owned())
}

fn cache_path(directory: &Path, url: &str) -> PathBuf {
    directory.join(format!("{:016x}.{}", hash_url(url), extension(url)))
}

fn extension(url: &str) -> &str {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let Some((_, extension)) = path.rsplit_once('.') else {
        return "jpg";
    };
    if ["jpg", "jpeg", "png", "webp"]
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
    {
        extension
    } else {
        "jpg"
    }
}

fn hash_url(url: &str) -> u64 {
    url.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn normalize_url(url: &str) -> Cow<'_, str> {
    for scheme in ["http://", "https://"] {
        let Some(rest) = url.strip_prefix(scheme) else {
            continue;
        };
        let Some((host, path)) = rest.split_once('/') else {
            continue;
        };
        let numbered = host
            .strip_prefix('p')
            .and_then(|host| host.strip_suffix(".music.126.net"))
            .is_some_and(|number| number.bytes().all(|byte| byte.is_ascii_digit()));
        if numbered {
            return Cow::Owned(format!("https://p1.music.126.net/{path}"));
        }
    }
    Cow::Borrowed(url)
}

fn local_path(url: &str) -> Option<String> {
    let path = if let Some(encoded) = url.strip_prefix("file://") {
        let encoded = encoded.strip_prefix("localhost").unwrap_or(encoded);
        PathBuf::from(percent_decode(encoded)?)
    } else {
        PathBuf::from(url)
    };
    path.is_absolute()
        .then_some(path)
        .filter(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex(bytes[index + 1])?;
            let low = hex(bytes[index + 2])?;
            decoded.push(high << 4 | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
