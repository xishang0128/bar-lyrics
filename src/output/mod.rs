mod json;

use crate::engine::Frame;
use crate::options::OutputKind;

pub(crate) trait Output {
    fn emit(&mut self, frame: &Frame) -> Result<(), String>;
}

pub(crate) fn create(kind: OutputKind) -> Box<dyn Output> {
    match kind {
        OutputKind::Json => Box::new(json::JsonLines),
    }
}
