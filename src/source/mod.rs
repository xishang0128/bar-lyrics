mod splayer;

use crate::model::{LyricsSnapshot, PlaybackSnapshot};
use crate::options::SourceKind;

pub(crate) trait Source {
    fn now_playing(&mut self) -> Result<PlaybackSnapshot, String>;
    fn lyrics(&mut self) -> Result<LyricsSnapshot, String>;
}

pub(crate) fn create(kind: SourceKind, endpoint: &str) -> Box<dyn Source> {
    match kind {
        SourceKind::Splayer => Box::new(splayer::SPlayer::new(endpoint)),
    }
}
