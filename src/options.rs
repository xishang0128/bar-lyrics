use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Control {
    Play,
    Pause,
    Toggle,
    Previous,
    Next,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SourceKind {
    Splayer,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OutputKind {
    Json,
    Waybar,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Alignment {
    Start,
    End,
}

impl Alignment {
    pub(crate) fn is_end(self) -> bool {
        self == Self::End
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SubtitleMode {
    Auto,
    Translation,
    Romanization,
    Hidden,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Options {
    pub(crate) source: SourceKind,
    pub(crate) source_endpoint: String,
    pub(crate) output: OutputKind,
    pub(crate) offset_ms: i64,
    pub(crate) max_chars: usize,
    pub(crate) alignment: Alignment,
    pub(crate) subtitle: SubtitleMode,
    pub(crate) inactive_opacity: u8,
    pub(crate) cover_dir: Option<PathBuf>,
    pub(crate) current_cover: Option<PathBuf>,
    pub(crate) waybar_signal: Option<u8>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            source: SourceKind::Splayer,
            source_endpoint: "http://127.0.0.1:14558".to_owned(),
            output: OutputKind::Json,
            offset_ms: 0,
            max_chars: 32,
            alignment: Alignment::Start,
            subtitle: SubtitleMode::Auto,
            inactive_opacity: 45,
            cover_dir: None,
            current_cover: None,
            waybar_signal: None,
        }
    }
}

impl Options {
    pub(crate) fn updated(&self, patch: serde_json::Value) -> Result<Self, String> {
        let patch = patch.as_object().ok_or("configuration must be an object")?;
        let mut value = serde_json::to_value(self).map_err(|error| error.to_string())?;
        value.as_object_mut().unwrap().extend(patch.clone());
        let mut options: Self = serde_json::from_value(value).map_err(|error| error.to_string())?;
        options.source_endpoint = options.source_endpoint.trim_end_matches('/').to_owned();
        if options
            .cover_dir
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty())
        {
            options.cover_dir = None;
        }
        if options
            .current_cover
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty())
        {
            options.current_cover = None;
        }
        let uri = options
            .source_endpoint
            .parse::<tungstenite::http::Uri>()
            .map_err(|error| error.to_string())?;
        if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.host().is_none() {
            return Err("source_endpoint must be an absolute HTTP(S) URL".to_owned());
        }
        if !(12..=80).contains(&options.max_chars)
            || !(10..=90).contains(&options.inactive_opacity)
            || !(-10000..=10000).contains(&options.offset_ms)
            || options
                .waybar_signal
                .is_some_and(|signal| !(1..=30).contains(&signal))
        {
            return Err("configuration value out of range".to_owned());
        }
        Ok(options)
    }
}
