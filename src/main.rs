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

const SOURCE_GRACE_PERIOD: Duration = Duration::from_secs(2);

fn run() -> Result<(), String> {
    let options = options::parse()?;
    let source = source::create(options.source, &options.source_endpoint);
    let mut output = output::create(
        options.output,
        options.inactive_opacity,
        options.waybar_signal,
    );
    let mut engine = Engine::new(source, options.cover_dir, options.current_cover);

    if options.once {
        engine.refresh()?;
        return output.emit(&engine.frame(
            options.offset_ms,
            options.max_chars,
            options.alignment,
            options.subtitle,
        ));
    }

    let mut next_poll = Instant::now();
    let mut refresh_failed_at = None;
    let mut last_frame = None;
    loop {
        if Instant::now() >= next_poll {
            match engine.refresh() {
                Ok(()) => refresh_failed_at = None,
                Err(_) => {
                    let failed_at = *refresh_failed_at.get_or_insert_with(Instant::now);
                    if failed_at.elapsed() >= SOURCE_GRACE_PERIOD {
                        engine.clear_playback();
                    }
                }
            }
            next_poll = Instant::now() + options.poll_interval;
        }

        let frame = engine.frame(
            options.offset_ms,
            options.max_chars,
            options.alignment,
            options.subtitle,
        );
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
