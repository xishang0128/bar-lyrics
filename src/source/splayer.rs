use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

use super::Source;
use crate::model::{LyricLine, LyricWord, LyricsSnapshot, PlaybackSnapshot, Track};

pub(super) struct SPlayer {
    base_url: String,
    agent: ureq::Agent,
}

impl SPlayer {
    pub(super) fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_owned(),
            agent: ureq::Agent::new_with_defaults(),
        }
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
    start_time: i64,
    #[serde(default)]
    end_time: i64,
    #[serde(default, rename = "isBG")]
    is_background: bool,
}

impl From<LyricLineResponse> for LyricLine {
    fn from(line: LyricLineResponse) -> Self {
        Self {
            words: line.words.into_iter().map(LyricWord::from).collect(),
            text: line.text,
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
