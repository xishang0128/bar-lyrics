use std::io::{self, Write};

use serde::Serialize;

use super::Output;
use crate::engine::{Frame, TextDisplay};
use crate::lyrics::TextSegment;

const MAX_RISE: f64 = 6.0 * 1_024.0;

pub(super) struct Waybar {
    inactive_opacity: u8,
    signal: Option<u8>,
    cover_path: String,
    last: Option<Payload>,
}

#[derive(Clone, PartialEq, Serialize)]
struct Payload {
    text: String,
    tooltip: String,
    class: Vec<&'static str>,
    alt: &'static str,
}

impl Payload {
    fn from_frame(frame: &Frame, inactive_opacity: u8) -> Self {
        if !frame.is_visible() {
            return Self {
                text: String::new(),
                tooltip: String::new(),
                class: vec!["hidden"],
                alt: "hidden",
            };
        }

        let state = if frame.is_playing() {
            "playing"
        } else {
            "paused"
        };
        let content = if frame.has_lyrics() {
            "lyrics"
        } else {
            "fallback"
        };
        let mut class = vec![state, content];
        if frame.is_transitioning() {
            class.push("transitioning");
        }
        let mut tooltip = escape_markup(frame.title());
        if !frame.artist().is_empty() {
            tooltip.push_str(" — ");
            tooltip.push_str(&escape_markup(frame.artist()));
        }
        if !frame.source().is_empty() {
            tooltip.push('\r');
            tooltip.push_str(&escape_markup(frame.source()));
        }

        Self {
            text: if frame.has_lyrics() {
                markup_segments(frame, inactive_opacity)
            } else {
                escape_markup(&frame.display_text())
            },
            tooltip,
            class,
            alt: state,
        }
    }
}

impl Waybar {
    pub(super) fn new(inactive_opacity: u8, signal: Option<u8>) -> Self {
        Self {
            inactive_opacity,
            signal,
            cover_path: String::new(),
            last: None,
        }
    }
}

impl Output for Waybar {
    fn emit(&mut self, frame: &Frame) -> Result<(), String> {
        if self.cover_path != frame.cover_path() {
            self.cover_path = frame.cover_path().to_owned();
            if let Some(signal) = self.signal {
                notify_waybar(signal);
            }
        }
        let payload = Payload::from_frame(frame, self.inactive_opacity);
        if self.last.as_ref() == Some(&payload) {
            return Ok(());
        }

        let stdout = io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer(&mut output, &payload).map_err(|error| error.to_string())?;
        writeln!(output).map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
        self.last = Some(payload);
        Ok(())
    }
}

fn notify_waybar(signal: u8) {
    let Some(pid) = waybar_ancestor() else {
        return;
    };
    // SAFETY: `pid` was read from procfs and `kill` is called without pointers.
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGRTMIN() + i32::from(signal));
    }
}

fn waybar_ancestor() -> Option<u32> {
    let mut pid = std::process::id();
    for _ in 0..4 {
        let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        let name = status
            .lines()
            .find_map(|line| line.strip_prefix("Name:\t"))?;
        if name == "waybar" {
            return Some(pid);
        }
        pid = status
            .lines()
            .find_map(|line| line.strip_prefix("PPid:\t"))?
            .trim()
            .parse()
            .ok()?;
        if pid == 0 {
            return None;
        }
    }
    None
}

fn markup_segments(frame: &Frame, inactive_opacity: u8) -> String {
    let mut markup = String::new();
    let (segments, transition_opacity, phase) = select_line(frame, inactive_opacity);
    let active_opacity = (transition_opacity * 100.0).round().clamp(1.0, 100.0) as u8;
    let rise = match phase {
        TextPhase::Static => 0,
        TextPhase::Outgoing => ((1.0 - transition_opacity) * MAX_RISE).round() as i32,
        TextPhase::Incoming => -((1.0 - transition_opacity) * MAX_RISE).round() as i32,
    };
    for segment in segments {
        let text = escape_markup(segment.as_str());
        let lyric_opacity = f64::from(inactive_opacity)
            + (100.0 - f64::from(inactive_opacity)) * segment.progress();
        let opacity = (f64::from(active_opacity) * lyric_opacity / 100.0)
            .round()
            .clamp(1.0, 100.0) as u8;
        if opacity >= 100 && rise == 0 {
            markup.push_str(&text);
        } else {
            markup.push_str(&format!(
                "<span alpha=\"{opacity}%\" rise=\"{rise}\">{text}</span>"
            ));
        }
    }
    markup
}

enum TextPhase {
    Static,
    Outgoing,
    Incoming,
}

fn select_line(frame: &Frame, inactive_opacity: u8) -> (&[TextSegment], f64, TextPhase) {
    match frame.text_display() {
        TextDisplay::Static(segments) => (segments, 1.0, TextPhase::Static),
        TextDisplay::Transition {
            outgoing,
            outgoing_opacity,
            incoming,
            incoming_opacity,
        } => {
            let inactive_opacity = f64::from(inactive_opacity) / 100.0;
            let outgoing_strength = line_strength(outgoing, inactive_opacity) * outgoing_opacity;
            let incoming_strength = line_strength(incoming, inactive_opacity) * incoming_opacity;
            if outgoing_strength > incoming_strength {
                (outgoing, outgoing_opacity, TextPhase::Outgoing)
            } else {
                (incoming, incoming_opacity, TextPhase::Incoming)
            }
        }
    }
}

fn line_strength(segments: &[TextSegment], inactive_opacity: f64) -> f64 {
    if segments.iter().any(TextSegment::is_active) {
        1.0
    } else {
        inactive_opacity
    }
}

fn escape_markup(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\'' => escaped.push_str("&apos;"),
            '"' => escaped.push_str("&quot;"),
            character => escaped.push(character),
        }
    }
    escaped
}
