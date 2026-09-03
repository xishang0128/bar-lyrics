#[derive(Clone, Debug)]
pub(crate) struct PlaybackSnapshot {
    pub(crate) track: Option<Track>,
    pub(crate) position_ms: f64,
    pub(crate) playing: bool,
    pub(crate) speed: f64,
    pub(crate) lyric_offset_ms: i64,
    pub(crate) lyric_available: bool,
    pub(crate) lyric_line_count: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Track {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) artists: Vec<String>,
    pub(crate) cover: String,
}

pub(crate) struct LyricsSnapshot {
    pub(crate) track_id: String,
    pub(crate) lines: Vec<LyricLine>,
    pub(crate) word_synced: bool,
    pub(crate) source: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LyricLine {
    pub(crate) words: Vec<LyricWord>,
    pub(crate) text: String,
    pub(crate) translation: String,
    pub(crate) romanization: String,
    pub(crate) start_time: i64,
    pub(crate) end_time: i64,
    pub(crate) is_background: bool,
}

impl LyricLine {
    pub(crate) fn effective_end_time(&self) -> Option<i64> {
        let end_time = self
            .words
            .iter()
            .map(|word| word.end_time)
            .max()
            .unwrap_or_default()
            .max(self.end_time);
        (end_time > self.start_time).then_some(end_time)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LyricWord {
    pub(crate) word: String,
    pub(crate) start_time: i64,
    pub(crate) end_time: i64,
}
