use std::env;
use std::path::PathBuf;

use serde::Serialize;

const USAGE: &str = "Usage: bar-lyrics [--source splayer] [--source-endpoint URL] \
                    [--output json|waybar] [--offset-ms N] [--max-chars N] \
                    [--align start|end] \
                    [--subtitle auto|translation|romanization|hidden] \
                    [--inactive-opacity N] [--cover-dir PATH] \
                    [--current-cover PATH] [--waybar-signal N] [--once]";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    Splayer,
}

#[derive(Clone, Copy)]
pub(crate) enum OutputKind {
    Json,
    Waybar,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
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

#[derive(Clone, Copy)]
pub(crate) enum SubtitleMode {
    Auto,
    Translation,
    Romanization,
    Hidden,
}

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
    pub(crate) once: bool,
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
            once: false,
        }
    }
}

pub(crate) fn parse() -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--once" => options.once = true,
            "--source" => {
                options.source = match args.next().as_deref() {
                    Some("splayer") => SourceKind::Splayer,
                    _ => return Err("--source needs splayer".to_owned()),
                };
            }
            "--source-endpoint" | "--base-url" => {
                options.source_endpoint = args.next().ok_or("--source-endpoint needs a value")?;
            }
            "--output" => {
                options.output = match args.next().as_deref() {
                    Some("json") => OutputKind::Json,
                    Some("waybar") => OutputKind::Waybar,
                    _ => return Err("--output needs json or waybar".to_owned()),
                };
            }
            "--offset-ms" => {
                let value = args.next().ok_or("--offset-ms needs a value")?;
                options.offset_ms = value.parse::<i64>().map_err(|_| "invalid --offset-ms")?;
            }
            "--max-chars" => {
                let value = args.next().ok_or("--max-chars needs a value")?;
                let max_chars = value.parse::<usize>().map_err(|_| "invalid --max-chars")?;
                options.max_chars = max_chars.clamp(12, 80);
            }
            "--align" => {
                options.alignment = match args.next().as_deref() {
                    Some("start") => Alignment::Start,
                    Some("end") => Alignment::End,
                    _ => return Err("--align needs start or end".to_owned()),
                };
            }
            "--subtitle" => {
                options.subtitle = match args.next().as_deref() {
                    Some("auto") => SubtitleMode::Auto,
                    Some("translation") => SubtitleMode::Translation,
                    Some("romanization") => SubtitleMode::Romanization,
                    Some("hidden") => SubtitleMode::Hidden,
                    _ => {
                        return Err(
                            "--subtitle needs auto, translation, romanization, or hidden"
                                .to_owned(),
                        );
                    }
                };
            }
            "--inactive-opacity" => {
                let value = args.next().ok_or("--inactive-opacity needs a value")?;
                let opacity = value
                    .parse::<u8>()
                    .map_err(|_| "invalid --inactive-opacity")?;
                options.inactive_opacity = opacity.clamp(10, 90);
            }
            "--cover-dir" => {
                options.cover_dir = Some(PathBuf::from(
                    args.next().ok_or("--cover-dir needs a path")?,
                ));
            }
            "--current-cover" => {
                options.current_cover = Some(PathBuf::from(
                    args.next().ok_or("--current-cover needs a path")?,
                ));
            }
            "--waybar-signal" => {
                let value = args.next().ok_or("--waybar-signal needs a value")?;
                let signal = value.parse::<u8>().map_err(|_| "invalid --waybar-signal")?;
                if !(1..=30).contains(&signal) {
                    return Err("--waybar-signal needs a value from 1 to 30".to_owned());
                }
                options.waybar_signal = Some(signal);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    options.source_endpoint = options.source_endpoint.trim_end_matches('/').to_owned();
    Ok(options)
}
