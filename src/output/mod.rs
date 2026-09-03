mod json;
mod waybar;

use crate::engine::Frame;
use crate::options::OutputKind;

pub(crate) trait Output {
    fn emit(&mut self, frame: &Frame) -> Result<(), String>;
}

pub(crate) fn create(
    kind: OutputKind,
    inactive_opacity: u8,
    waybar_signal: Option<u8>,
) -> Box<dyn Output> {
    match kind {
        OutputKind::Json => Box::new(json::JsonLines),
        OutputKind::Waybar => Box::new(waybar::Waybar::new(inactive_opacity, waybar_signal)),
    }
}
