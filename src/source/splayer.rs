use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use tungstenite::{Message, connect};

use super::Source;
use crate::model::{LyricLine, LyricWord, LyricsSnapshot, PlaybackSnapshot, Track};

pub(super) struct SPlayer {
    base_url: String,
    agent: ureq::Agent,
    updates: Option<UpdateWatcher>,
}

impl SPlayer {
    pub(super) fn new(base_url: &str, watch: bool) -> Result<Self, String> {
        Ok(Self {
            base_url: base_url.to_owned(),
            agent: ureq::Agent::new_with_defaults(),
            updates: watch
                .then(|| UpdateWatcher::connect(base_url))
                .transpose()?,
        })
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{path}", self.base_url);
        let mut response = self
            .agent
            .get(&url)
            .call()
            .map_err(|error| format!("GET {url}: {error}"))?;
        response
            .body_mut()
            .read_json::<T>()
            .map_err(|error| format!("decode {url}: {error}"))
    }
}

impl Source for SPlayer {
    fn now_playing(&mut self) -> Result<PlaybackSnapshot, String> {
        let response: NowPlayingResponse = self.get("/api/now-playing")?;
        let clock_age = unix_millis()
            .saturating_sub(response.send_timestamp)
            .clamp(0, 5_000);
        let position_ms = if response.playing {
            response.position as f64 + clock_age as f64 * response.speed
        } else {
            response.position as f64
        };
        Ok(PlaybackSnapshot {
            track: response.track.map(Track::from),
            position_ms,
            playing: response.playing && response.state == "playing",
            speed: response.speed,
            lyric_offset_ms: response.lyric_offset_ms,
            lyric_available: response.lyric_available,
            lyric_line_count: response.lyric_line_count,
        })
    }

    fn lyrics(&mut self) -> Result<LyricsSnapshot, String> {
        let response: LyricsResponse = self.get("/api/lyrics")?;
        let source = response
            .source
            .as_ref()
            .map(LyricSource::label)
            .unwrap_or_else(|| "SPlayer".to_owned());
        let word_synced = response.supports_word_sync();
        Ok(LyricsSnapshot {
            track_id: response.track_id,
            lines: response.lyric.into_iter().map(LyricLine::from).collect(),
            word_synced,
            source,
        })
    }

    fn take_update(&mut self) -> Result<bool, String> {
        self.updates.as_mut().map_or(Ok(false), UpdateWatcher::take)
    }
}

const WS_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const WS_RECONNECT_DELAY: Duration = Duration::from_secs(1);

enum UpdateEvent {
    Connected,
    Changed,
    Disconnected(String),
}

struct UpdateWatcher {
    receiver: Receiver<UpdateEvent>,
    connected: bool,
    last_error: String,
}

impl UpdateWatcher {
    fn connect(base_url: &str) -> Result<Self, String> {
        let url = websocket_url(base_url)?;
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("splayer-websocket".to_owned())
            .spawn({
                let url = url.clone();
                move || watch_updates(&url, sender)
            })
            .map_err(|error| format!("start SPlayer WebSocket watcher: {error}"))?;

        match receiver.recv_timeout(WS_CONNECT_TIMEOUT) {
            Ok(UpdateEvent::Connected) => Ok(Self {
                receiver,
                connected: true,
                last_error: String::new(),
            }),
            Ok(UpdateEvent::Disconnected(error)) => Err(websocket_error(&url, &error)),
            Ok(UpdateEvent::Changed) => unreachable!("change received before WebSocket connection"),
            Err(RecvTimeoutError::Timeout) => Err(websocket_error(&url, "connection timed out")),
            Err(RecvTimeoutError::Disconnected) => {
                Err(websocket_error(&url, "watcher stopped unexpectedly"))
            }
        }
    }

    fn take(&mut self) -> Result<bool, String> {
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

fn websocket_url(base_url: &str) -> Result<String, String> {
    let base_url = base_url.trim_end_matches('/');
    if let Some(endpoint) = base_url.strip_prefix("http://") {
        return Ok(format!("ws://{endpoint}/ws"));
    }
    if let Some(endpoint) = base_url.strip_prefix("https://") {
        return Ok(format!("wss://{endpoint}/ws"));
    }
    Err("SPlayer endpoint must start with http:// or https://".to_owned())
}

fn websocket_error(url: &str, error: &str) -> String {
    format!(
        "SPlayer WebSocket unavailable at {url}; enable WebSocket in SPlayer external API settings: {error}"
    )
}

fn watch_updates(url: &str, sender: Sender<UpdateEvent>) {
    loop {
        let disconnected = match connect(url) {
            Ok((mut socket, _)) => {
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
            Err(error) => error.to_string(),
        };

        if sender
            .send(UpdateEvent::Disconnected(disconnected))
            .is_err()
        {
            return;
        }
        thread::sleep(WS_RECONNECT_DELAY);
    }
}

fn is_update(message: &str) -> bool {
    #[derive(Deserialize)]
    struct Envelope {
        kind: String,
    }

    serde_json::from_str::<Envelope>(message).is_ok_and(|message| message.kind == "event")
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NowPlayingResponse {
    #[serde(default)]
    track: Option<TrackResponse>,
    #[serde(default)]
    position: i64,
    #[serde(default)]
    playing: bool,
    #[serde(default)]
    state: String,
    #[serde(default = "default_speed")]
    speed: f64,
    #[serde(default)]
    lyric_offset_ms: i64,
    #[serde(default)]
    lyric_available: bool,
    #[serde(default)]
    lyric_line_count: usize,
    #[serde(default)]
    send_timestamp: i64,
}

#[derive(Deserialize)]
struct TrackResponse {
    #[serde(default, deserialize_with = "string_from_any")]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    artists: Vec<ArtistResponse>,
    #[serde(default)]
    cover: String,
}

impl From<TrackResponse> for Track {
    fn from(track: TrackResponse) -> Self {
        Self {
            id: track.id,
            title: track.title,
            artists: track
                .artists
                .into_iter()
                .map(|artist| artist.name)
                .collect(),
            cover: track.cover,
        }
    }
}

#[derive(Deserialize)]
struct ArtistResponse {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LyricsResponse {
    #[serde(default, deserialize_with = "string_from_any")]
    track_id: String,
    #[serde(default)]
    lyric: Vec<LyricLineResponse>,
    #[serde(default)]
    source: Option<LyricSource>,
}

impl LyricsResponse {
    fn supports_word_sync(&self) -> bool {
        let timed_format = self.source.as_ref().is_some_and(|source| {
            matches!(
                source.format.to_ascii_lowercase().as_str(),
                "qrc" | "yrc" | "ttml"
            )
        });
        timed_format
            || self.lyric.iter().any(|line| {
                line.words.windows(2).any(|words| {
                    words[0].start_time != words[1].start_time
                        || words[0].end_time != words[1].end_time
                })
            })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LyricLineResponse {
    #[serde(default)]
    words: Vec<LyricWordResponse>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    translated_lyric: String,
    #[serde(default)]
    roman_lyric: String,
    #[serde(default)]
    start_time: i64,
    #[serde(default)]
    end_time: i64,
    #[serde(default, rename = "isBG")]
    is_background: bool,
}

impl From<LyricLineResponse> for LyricLine {
    fn from(line: LyricLineResponse) -> Self {
        let romanization = if line.roman_lyric.trim().is_empty() {
            line.words
                .iter()
                .map(|word| word.roman_word.as_str())
                .collect()
        } else {
            line.roman_lyric
        };
        Self {
            words: line.words.into_iter().map(LyricWord::from).collect(),
            text: line.text,
            translation: line.translated_lyric,
            romanization,
            start_time: line.start_time,
            end_time: line.end_time,
            is_background: line.is_background,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LyricWordResponse {
    #[serde(default)]
    word: String,
    #[serde(default)]
    roman_word: String,
    #[serde(default)]
    start_time: i64,
    #[serde(default)]
    end_time: i64,
}

impl From<LyricWordResponse> for LyricWord {
    fn from(word: LyricWordResponse) -> Self {
        Self {
            word: word.word,
            start_time: word.start_time,
            end_time: word.end_time,
        }
    }
}

#[derive(Deserialize)]
struct LyricSource {
    #[serde(default)]
    source: String,
    #[serde(default)]
    platform: String,
    #[serde(default)]
    format: String,
}

impl LyricSource {
    fn label(&self) -> String {
        if !self.platform.is_empty() {
            format!("SPlayer · {}", self.platform)
        } else if !self.format.is_empty() {
            format!("SPlayer · {}", self.format)
        } else if !self.source.is_empty() {
            format!("SPlayer · {}", self.source)
        } else {
            "SPlayer".to_owned()
        }
    }
}

fn default_speed() -> f64 {
    1.0
}

fn string_from_any<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(value) => value,
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Null => String::new(),
        value => value.to_string(),
    })
}

fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}
