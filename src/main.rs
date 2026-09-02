mod cover;
mod engine;
mod lyrics;
mod model;
mod options;
mod output;
mod source;

use std::thread;
use std::time::{Duration, Instant};

use engine::Engine;

fn run() -> Result<(), String> {
    let options = options::parse()?;
    let source = source::create(options.source, &options.source_endpoint);
    let mut output = output::create(options.output);
    let mut engine = Engine::new(source, options.cover_dir);

    if options.once {
        engine.refresh()?;
        return output.emit(&engine.frame(options.offset_ms, options.max_chars, options.alignment));
    }

    let mut next_poll = Instant::now();
    let mut last_frame = None;
    loop {
        if Instant::now() >= next_poll {
            if engine.refresh().is_err() {
                engine.clear_playback();
            }
            next_poll = Instant::now() + options.poll_interval;
        }

        let frame = engine.frame(options.offset_ms, options.max_chars, options.alignment);
        if last_frame.as_ref() != Some(&frame) {
            output.emit(&frame)?;
            last_frame = Some(frame);
        }
        thread::sleep(Duration::from_millis(16));
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("bar-lyrics: {error}");
        std::process::exit(1);
    }
}
