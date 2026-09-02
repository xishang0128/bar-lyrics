use std::env;
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;

const USAGE: &str = "Usage: bar-lyrics [--source splayer] [--source-endpoint URL] \
                    [--output json] [--poll-ms N] \
                    [--offset-ms N] [--max-chars N] [--align start|end] \
                    [--cover-dir PATH] [--once]";

#[derive(Clone, Copy)]
pub(crate) enum SourceKind {
    Splayer,
}

#[derive(Clone, Copy)]
pub(crate) enum OutputKind {
    Json,
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

pub(crate) struct Options {
    pub(crate) source: SourceKind,
    pub(crate) source_endpoint: String,
    pub(crate) output: OutputKind,
    pub(crate) poll_interval: Duration,
    pub(crate) offset_ms: i64,
    pub(crate) max_chars: usize,
    pub(crate) alignment: Alignment,
    pub(crate) cover_dir: Option<PathBuf>,
    pub(crate) once: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            source: SourceKind::Splayer,
            source_endpoint: "http://127.0.0.1:14558".to_owned(),
            output: OutputKind::Json,
            poll_interval: Duration::from_millis(250),
            offset_ms: 0,
            max_chars: 32,
            alignment: Alignment::Start,
            cover_dir: None,
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
                    _ => return Err("--output needs json".to_owned()),
                };
            }
            "--poll-ms" => {
                let value = args.next().ok_or("--poll-ms needs a value")?;
                let millis = value.parse::<u64>().map_err(|_| "invalid --poll-ms")?;
                options.poll_interval = Duration::from_millis(millis.clamp(100, 2_000));
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
            "--cover-dir" => {
                options.cover_dir = Some(PathBuf::from(
                    args.next().ok_or("--cover-dir needs a path")?,
                ));
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
