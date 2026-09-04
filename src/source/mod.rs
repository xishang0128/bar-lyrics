mod splayer;

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
    match kind {
        SourceKind::Splayer => Ok(Box::new(splayer::SPlayer::new(endpoint, watch)?)),
    }
}
