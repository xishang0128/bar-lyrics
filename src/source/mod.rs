mod mpris;
mod splayer;

use std::net::IpAddr;
use std::sync::mpsc::{self, Receiver, SyncSender};

use crate::model::{LyricsSnapshot, PlaybackSnapshot};
use crate::options::SourceKind;

pub(crate) trait Source {
    fn now_playing(&mut self) -> Result<PlaybackSnapshot, String>;
    fn lyrics(&mut self) -> Result<LyricsSnapshot, String>;
    fn take_update(&mut self) -> Result<bool, String>;
}

pub(crate) fn create(
    kind: SourceKind,
    endpoint: &str,
    watch: bool,
) -> Result<Box<dyn Source>, String> {
    if watch && is_local_endpoint(endpoint) {
        return Ok(Box::new(SourceRouter {
            kind,
            endpoint: endpoint.to_owned(),
            events: mpris::subscribe(),
            active: None,
        }));
    }
    open(kind, endpoint, watch.then(|| mpsc::sync_channel(1).1))
}

fn open(
    kind: SourceKind,
    endpoint: &str,
    reconnect: Option<Receiver<()>>,
) -> Result<Box<dyn Source>, String> {
    match kind {
        SourceKind::Splayer => Ok(Box::new(splayer::SPlayer::new(endpoint, reconnect)?)),
    }
}

fn player_source(name: &str) -> Option<SourceKind> {
    let player = name.strip_prefix("org.mpris.MediaPlayer2.")?;
    match player.split('.').next()? {
        "splayer_next" => Some(SourceKind::Splayer),
        _ => None,
    }
}

fn is_local_endpoint(endpoint: &str) -> bool {
    endpoint
        .parse::<tungstenite::http::Uri>()
        .ok()
        .and_then(|uri| uri.host().map(str::to_owned))
        .is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost")
                || host
                    .trim_matches(['[', ']'])
                    .parse::<IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        })
}

struct ActiveSource {
    source: Box<dyn Source>,
    reconnect: SyncSender<()>,
}

struct SourceRouter {
    kind: SourceKind,
    endpoint: String,
    events: Receiver<mpris::Event>,
    active: Option<ActiveSource>,
}

impl SourceRouter {
    fn activate(&mut self, kind: SourceKind) -> Result<(), String> {
        if let Some(active) = &self.active {
            let _ = active.reconnect.try_send(());
        } else {
            let (reconnect, events) = mpsc::sync_channel(1);
            self.active = Some(ActiveSource {
                source: open(kind, &self.endpoint, Some(events))?,
                reconnect,
            });
        }
        Ok(())
    }

    fn current(&mut self) -> Result<&mut (dyn Source + '_), String> {
        match &mut self.active {
            Some(active) => Ok(active.source.as_mut()),
            None => Err("waiting for a supported player".to_owned()),
        }
    }
}

impl Source for SourceRouter {
    fn now_playing(&mut self) -> Result<PlaybackSnapshot, String> {
        self.current()?.now_playing()
    }

    fn lyrics(&mut self) -> Result<LyricsSnapshot, String> {
        self.current()?.lyrics()
    }

    fn take_update(&mut self) -> Result<bool, String> {
        while let Ok(event) = self.events.try_recv() {
            match event {
                mpris::Event::Activity(player) => {
                    if let Some(kind) = player_source(&player).filter(|kind| *kind == self.kind) {
                        self.activate(kind)?;
                    }
                }
                mpris::Event::Unavailable(error) => {
                    eprintln!("bar-lyrics: MPRIS unavailable, using configured source: {error}");
                    self.activate(self.kind)?;
                }
            }
        }
        self.current()?.take_update()
    }
}
