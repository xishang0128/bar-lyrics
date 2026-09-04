use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use crate::model::LyricLine;
use crate::options::SubtitleMode;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct TextSegment {
    text: String,
    active: bool,
    progress: f64,
}

impl TextSegment {
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn progress(&self) -> f64 {
        self.progress
    }
}

#[derive(Clone)]
struct DisplayGrapheme {
    text: String,
    active: bool,
    progress: f64,
}

struct TimedGrapheme {
    text: String,
    start_ms: i64,
    end_ms: i64,
}

pub(crate) fn display_segments(
    line: &LyricLine,
    position_ms: i64,
    word_synced: bool,
    max_chars: usize,
    align_end: bool,
) -> Option<Vec<TextSegment>> {
    let timed = if word_synced {
        timed_graphemes(line)
    } else {
        Vec::new()
    };
    let (mut graphemes, active_index) = if !timed.is_empty() {
        let active_index = timed
            .iter()
            .position(|item| position_ms < item.end_ms)
            .unwrap_or(timed.len());
        let graphemes = timed
            .into_iter()
            .map(|item| DisplayGrapheme {
                active: position_ms >= item.start_ms,
                progress: timed_progress(position_ms, item.start_ms, item.end_ms),
                text: item.text,
            })
            .collect::<Vec<_>>();
        (graphemes, Some(active_index))
    } else {
        (plain_graphemes(line), None)
    };

    if graphemes.is_empty() {
        return None;
    }
    if let Some(active_index) = active_index {
        crop_synced(&mut graphemes, active_index, max_chars);
    } else {
        crop_plain(&mut graphemes, max_chars, align_end);
    }
    Some(merge_segments(graphemes))
}

pub(crate) fn truncate_text(text: &str, max_chars: usize) -> String {
    let mut graphemes = UnicodeSegmentation::graphemes(text, true)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if graphemes.len() > max_chars {
        graphemes.truncate(max_chars);
        graphemes[max_chars - 1] = "…".to_owned();
    }
    graphemes.concat()
}

pub(crate) fn subtitle(
    line: &LyricLine,
    position_ms: i64,
    word_synced: bool,
    max_chars: usize,
    mode: SubtitleMode,
) -> String {
    let translation = line.translation.trim();
    let romanization = line.romanization.trim();
    let (text, follows_lyrics) = match mode {
        SubtitleMode::Auto if !translation.is_empty() => (translation, false),
        SubtitleMode::Auto => (romanization, true),
        SubtitleMode::Translation => (translation, false),
        SubtitleMode::Romanization => (romanization, true),
        SubtitleMode::Hidden => ("", false),
    };
    if !follows_lyrics || !word_synced {
        return truncate_text(text, max_chars);
    }

    let timed = timed_graphemes(line);
    let active_index = timed
        .iter()
        .position(|item| position_ms < item.end_ms)
        .unwrap_or(timed.len());
    truncate_around(text, max_chars, active_index, timed.len())
}

fn truncate_around(text: &str, max_chars: usize, focus: usize, source_len: usize) -> String {
    let mut graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<_>>();
    let len = graphemes.len();
    if len <= max_chars || source_len == 0 {
        return text.to_owned();
    }

    let focus = focus.saturating_mul(len) / source_len;
    let first = focus.saturating_sub(max_chars / 2).min(len - max_chars);
    graphemes = graphemes[first..first + max_chars].to_vec();
    if first > 0 {
        graphemes[0] = "…";
    }
    if first + max_chars < len {
        graphemes[max_chars - 1] = "…";
    }
    graphemes.concat()
}

fn line_text(line: &LyricLine) -> String {
    if line.text.is_empty() {
        line.words.iter().map(|word| word.word.as_str()).collect()
    } else {
        line.text.clone()
    }
}

fn plain_graphemes(line: &LyricLine) -> Vec<DisplayGrapheme> {
    UnicodeSegmentation::graphemes(line_text(line).as_str(), true)
        .map(|text| DisplayGrapheme {
            text: text.to_owned(),
            active: true,
            progress: 1.0,
        })
        .collect()
}

fn timed_progress(position_ms: i64, start_ms: i64, end_ms: i64) -> f64 {
    let duration = (end_ms - start_ms).max(1) as f64;
    ((position_ms - start_ms) as f64 / duration).clamp(0.0, 1.0)
}

fn timed_graphemes(line: &LyricLine) -> Vec<TimedGrapheme> {
    let mut result = Vec::new();
    for word in &line.words {
        append_timed_text(&mut result, &word.word, word.start_time, word.end_time);
    }
    result
}

fn append_timed_text(result: &mut Vec<TimedGrapheme>, text: &str, start_ms: i64, end_ms: i64) {
    let graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<_>>();
    let count = graphemes
        .len()
        .saturating_sub(text.matches(' ').count())
        .max(1) as i64;

    let duration = (end_ms - start_ms).max(0);
    let mut index = 0;
    for grapheme in graphemes {
        let start = start_ms + duration * index / count;
        if grapheme != " " {
            index += 1;
        }
        let end = start_ms + duration * index / count;
        result.push(TimedGrapheme {
            text: if grapheme == " " { "\u{a0}" } else { grapheme }.to_owned(),
            start_ms: start,
            end_ms: end.max(start + 1),
        });
    }
}

fn crop_synced(graphemes: &mut Vec<DisplayGrapheme>, active_index: usize, max_chars: usize) {
    if graphemes.len() <= max_chars {
        return;
    }

    let first = active_index
        .saturating_sub(max_chars / 2)
        .min(graphemes.len() - max_chars);
    let clipped_right = first + max_chars < graphemes.len();
    *graphemes = graphemes[first..first + max_chars].to_vec();
    if first > 0 {
        graphemes[0] = DisplayGrapheme {
            text: "…".to_owned(),
            active: true,
            progress: 1.0,
        };
    }
    if clipped_right {
        graphemes[max_chars - 1] = DisplayGrapheme {
            text: "…".to_owned(),
            active: false,
            progress: 0.0,
        };
    }
}

fn crop_plain(graphemes: &mut Vec<DisplayGrapheme>, max_chars: usize, align_end: bool) {
    if graphemes.len() <= max_chars {
        return;
    }

    if align_end {
        *graphemes = graphemes[graphemes.len() - max_chars..].to_vec();
        graphemes[0].text = "…".to_owned();
    } else {
        graphemes.truncate(max_chars);
        graphemes[max_chars - 1].text = "…".to_owned();
    }
}

fn merge_segments(graphemes: Vec<DisplayGrapheme>) -> Vec<TextSegment> {
    let mut segments: Vec<TextSegment> = Vec::new();
    for grapheme in graphemes {
        if let Some(segment) = segments.last_mut().filter(|segment| {
            segment.active == grapheme.active && segment.progress == grapheme.progress
        }) {
            segment.text.push_str(&grapheme.text);
        } else {
            segments.push(TextSegment {
                text: grapheme.text,
                active: grapheme.active,
                progress: grapheme.progress,
            });
        }
    }
    segments
}
