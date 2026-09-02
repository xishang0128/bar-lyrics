use std::io::{self, Write};

use super::Output;
use crate::engine::Frame;

pub(super) struct JsonLines;

impl Output for JsonLines {
    fn emit(&mut self, frame: &Frame) -> Result<(), String> {
        let stdout = io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer(&mut output, frame).map_err(|error| error.to_string())?;
        writeln!(output).map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())
    }
}
