use std::time::Instant;

use serde::Serialize;

use crate::cover::CoverCache;
use crate::lyrics;
use crate::model::{LyricLine, PlaybackSnapshot, Track};
use crate::options::{Alignment, SubtitleMode};
use crate::source::Source;

const LINE_TRANSITION_MAX_LEAD_MS: i64 = 320;

pub(crate) enum TextDisplay<'a> {
    Static {
        segments: &'a [lyrics::TextSegment],
        subtitle: &'a str,
    },
    Transition {
        outgoing: &'a [lyrics::TextSegment],
        outgoing_subtitle: &'a str,
        outgoing_opacity: f64,
        incoming: &'a [lyrics::TextSegment],
        incoming_subtitle: &'a str,
        incoming_opacity: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct DisplayLine {
    key: String,
    segments: Vec<lyrics::TextSegment>,
    subtitle: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct LineTransition {
    outgoing: DisplayLine,
    outgoing_height_factor: f64,
    outgoing_opacity: f64,
    incoming_opacity: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Frame {
    visible: bool,
    playing: bool,
    title: String,
    artist: String,
    source: String,
    cover_path: String,
    fallback_text: String,
    alignment: Alignment,
    line: Option<DisplayLine>,
    transition: Option<LineTransition>,
}

impl Frame {
    pub(crate) fn display_text(&self) -> String {
        self.fallback_text.clone()
    }

    pub(crate) fn text_display(&self) -> TextDisplay<'_> {
        if let Some(transition) = &self.transition {
            if let Some(line) = &self.line {
                return TextDisplay::Transition {
                    outgoing: &transition.outgoing.segments,
                    outgoing_subtitle: &transition.outgoing.subtitle,
                    outgoing_opacity: transition.outgoing_opacity,
                    incoming: &line.segments,
                    incoming_subtitle: &line.subtitle,
                    incoming_opacity: transition.incoming_opacity,
                };
            }
        }
        let line = self.line.as_ref();
        TextDisplay::Static {
            segments: line
                .map(|line| line.segments.as_slice())
                .unwrap_or_default(),
            subtitle: line.map(|line| line.subtitle.as_str()).unwrap_or_default(),
        }
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(crate) fn has_lyrics(&self) -> bool {
        self.line.is_some()
    }

    pub(crate) fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn artist(&self) -> &str {
        &self.artist
    }

    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn cover_path(&self) -> &str {
        &self.cover_path
    }

    fn hidden(alignment: Alignment) -> Self {
        Self {
            visible: false,
            playing: false,
            title: String::new(),
            artist: String::new(),
            source: String::new(),
            cover_path: String::new(),
            fallback_text: String::new(),
            alignment,
            line: None,
            transition: None,
        }
    }

    fn fallback(
        playback: &Playback,
        track: &Track,
        artist: String,
        source: &str,
        cover_path: &str,
        max_chars: usize,
        alignment: Alignment,
    ) -> Self {
        let fallback_text = lyrics::truncate_text(&track.title, max_chars);
        Self {
            visible: !fallback_text.is_empty(),
            playing: playback.playing,
            title: track.title.clone(),
            artist,
            source: source.to_owned(),
            cover_path: cover_path.to_owned(),
            fallback_text,
            alignment,
            line: None,
            transition: None,
        }
    }
}

struct Playback {
    track: Option<Track>,
    position_ms: f64,
    anchored_at: Instant,
    playing: bool,
    speed: f64,
    lyric_offset_ms: i64,
}

impl Playback {
    fn from_snapshot(snapshot: PlaybackSnapshot) -> Self {
        Self {
            track: snapshot.track,
            position_ms: snapshot.position_ms,
            anchored_at: Instant::now(),
            playing: snapshot.playing,
            speed: snapshot.speed,
            lyric_offset_ms: snapshot.lyric_offset_ms,
        }
    }

    fn current_position_ms(&self, extra_offset_ms: i64) -> i64 {
        let elapsed = if self.playing {
            self.anchored_at.elapsed().as_secs_f64() * 1_000.0 * self.speed
        } else {
            0.0
        };
        (self.position_ms + elapsed).round() as i64 + self.lyric_offset_ms + extra_offset_ms
    }
}

pub(crate) struct Engine {
    source: Box<dyn Source>,
    playback: Option<Playback>,
    lyrics_track_id: String,
    lyrics: Vec<LyricLine>,
    lyrics_word_synced: bool,
    lyrics_source: String,
    cover_cache: CoverCache,
}

impl Engine {
    pub(crate) fn new(
        source: Box<dyn Source>,
        cover_dir: Option<std::path::PathBuf>,
        current_cover: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            source,
            playback: None,
            lyrics_track_id: String::new(),
            lyrics: Vec::new(),
            lyrics_word_synced: false,
            lyrics_source: String::new(),
            cover_cache: CoverCache::new(cover_dir, current_cover),
        }
    }

    pub(crate) fn clear_playback(&mut self) {
        self.playback = None;
        self.cover_cache.set_url("");
    }

    pub(crate) fn set_cover_cache(
        &mut self,
        directory: Option<std::path::PathBuf>,
        current: Option<std::path::PathBuf>,
    ) {
        self.cover_cache = CoverCache::new(directory, current);
        let url = self
            .playback
            .as_ref()
            .and_then(|playback| playback.track.as_ref())
            .map(|track| track.cover.as_str())
            .unwrap_or("");
        self.cover_cache.set_url(url);
    }

    pub(crate) fn take_source_update(&mut self) -> Result<bool, String> {
        self.source.take_update()
    }

    pub(crate) fn refresh(&mut self) -> Result<(), String> {
        let snapshot = self.source.now_playing()?;
        self.cover_cache.set_url(
            snapshot
                .track
                .as_ref()
                .map(|track| track.cover.as_str())
                .unwrap_or(""),
        );
        let next_track_id = snapshot
            .track
            .as_ref()
            .map(|track| track.id.as_str())
            .unwrap_or("");
        let track_changed = next_track_id != self.lyrics_track_id;
        let lyrics_changed = snapshot.lyric_available
            && snapshot.lyric_line_count > 0
            && snapshot.lyric_line_count != self.lyrics.len();

        if track_changed {
            self.lyrics_track_id.clear();
            self.lyrics.clear();
            self.lyrics_word_synced = false;
            self.lyrics_source.clear();
        }

        if !next_track_id.is_empty()
            && snapshot.lyric_available
            && (track_changed || lyrics_changed)
        {
            let response = self.source.lyrics()?;
            if response.track_id == next_track_id {
                self.lyrics_word_synced = response.word_synced;
                self.lyrics_track_id = response.track_id;
                self.lyrics = response.lines;
                self.lyrics_source = response.source;
            }
        } else if !snapshot.lyric_available {
            self.lyrics_track_id = next_track_id.to_owned();
            self.lyrics.clear();
            self.lyrics_word_synced = false;
            self.lyrics_source.clear();
        }

        self.playback = Some(Playback::from_snapshot(snapshot));
        Ok(())
    }

    pub(crate) fn frame(
        &mut self,
        offset_ms: i64,
        max_chars: usize,
        alignment: Alignment,
        subtitle_mode: SubtitleMode,
    ) -> Frame {
        let cover_path = self.cover_cache.path().to_owned();
        let Some(playback) = &self.playback else {
            return Frame::hidden(alignment);
        };
        let Some(track) = &playback.track else {
            return Frame::hidden(alignment);
        };

        let artist = track
            .artists
            .iter()
            .map(String::as_str)
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
            .join(" / ");
        let position_ms = playback.current_position_ms(offset_ms);
        let playing = playback.playing;
        let lines = self
            .lyrics
            .iter()
            .filter(|line| !line.is_background)
            .collect::<Vec<_>>();
        let Some(line_index) = lines
            .iter()
            .rposition(|line| line.start_time <= position_ms)
        else {
            return Frame::fallback(
                playback,
                track,
                artist,
                &self.lyrics_source,
                &cover_path,
                max_chars,
                alignment,
            );
        };

        let mut incoming_index = line_index;
        let mut transition_state = None;
        if playing && line_index > 0 {
            let start = transition_start(lines[line_index - 1], lines[line_index]);
            if position_ms >= start && position_ms < start + LINE_TRANSITION_MAX_LEAD_MS {
                transition_state = Some((line_index - 1, animation_progress(start, position_ms)));
            }
        }
        if playing && transition_state.is_none() && line_index + 1 < lines.len() {
            let start = transition_start(lines[line_index], lines[line_index + 1]);
            if position_ms >= start {
                incoming_index += 1;
                transition_state = Some((line_index, animation_progress(start, position_ms)));
            }
        }

        let Some(line) = display_line(
            lines[incoming_index],
            &track.title,
            position_ms,
            self.lyrics_word_synced,
            max_chars,
            alignment,
            subtitle_mode,
        ) else {
            return Frame::fallback(
                playback,
                track,
                artist,
                &self.lyrics_source,
                &cover_path,
                max_chars,
                alignment,
            );
        };
        let transition = transition_state.and_then(|(outgoing_index, progress)| {
            display_line(
                lines[outgoing_index],
                &track.title,
                position_ms,
                self.lyrics_word_synced,
                max_chars,
                alignment,
                subtitle_mode,
            )
            .map(|outgoing| line_transition(outgoing, progress))
        });

        Frame {
            visible: true,
            playing,
            title: track.title.clone(),
            artist,
            source: self.lyrics_source.clone(),
            cover_path,
            fallback_text: lyrics::truncate_text(&track.title, max_chars),
            alignment,
            line: Some(line),
            transition,
        }
    }
}

fn transition_start(line: &LyricLine, next_line: &LyricLine) -> i64 {
    let earliest = next_line
        .start_time
        .saturating_sub(LINE_TRANSITION_MAX_LEAD_MS);
    line.effective_end_time()
        .unwrap_or(earliest)
        .clamp(earliest, next_line.start_time)
}

fn animation_progress(start_ms: i64, position_ms: i64) -> f64 {
    ((position_ms - start_ms) as f64 / LINE_TRANSITION_MAX_LEAD_MS as f64).clamp(0.0, 1.0)
}

fn display_line(
    line: &LyricLine,
    title: &str,
    position_ms: i64,
    word_synced: bool,
    max_chars: usize,
    alignment: Alignment,
    subtitle_mode: SubtitleMode,
) -> Option<DisplayLine> {
    Some(DisplayLine {
        key: format!("{title}|{}", line.start_time),
        segments: lyrics::display_segments(
            line,
            position_ms,
            word_synced,
            max_chars,
            alignment.is_end(),
        )?,
        subtitle: lyrics::subtitle(line, position_ms, word_synced, max_chars, subtitle_mode),
    })
}

fn line_transition(outgoing: DisplayLine, progress: f64) -> LineTransition {
    let eased = 1.0 - (1.0 - progress).powi(3);
    LineTransition {
        outgoing,
        outgoing_height_factor: 1.0 - eased,
        outgoing_opacity: (1.0 - progress * 1.7).max(0.0),
        incoming_opacity: (0.25 + progress * 1.5).min(1.0),
    }
}
