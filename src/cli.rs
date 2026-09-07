use std::env;
use std::path::PathBuf;

use crate::options::*;

const USAGE: &str = "Usage: bar-lyrics (JSON-RPC IPC; optional config.update on stdin)\n\
                    Legacy CLI: bar-lyrics [--source splayer] [--source-endpoint URL] \
                    [--output json|waybar] [--offset-ms N] [--max-chars N] \
                    [--align start|end] \
                    [--subtitle auto|translation|romanization|hidden] \
                    [--inactive-opacity N] [--cover-dir PATH] \
                    [--current-cover PATH] [--waybar-signal N] [--once] \
                    [--control play|pause|toggle|previous|next]";

pub(crate) struct Launch {
    pub(crate) options: Options,
    pub(crate) once: bool,
    pub(crate) control: Option<Control>,
}

pub(crate) fn parse() -> Result<Launch, String> {
    let mut options = Options::default();
    let mut once = false;
    let mut control = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--control" => {
                control = Some(match args.next().as_deref() {
                    Some("play") => Control::Play,
                    Some("pause") => Control::Pause,
                    Some("toggle") => Control::Toggle,
                    Some("previous") => Control::Previous,
                    Some("next") => Control::Next,
                    _ => {
                        return Err(
                            "--control needs play, pause, toggle, previous, or next".to_owned()
                        );
                    }
                });
            }
            "--once" => once = true,
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
    Ok(Launch {
        options,
        once,
        control,
    })
}
