use std::thread;
use std::time::{Duration, Instant};

use crate::engine::{Engine, Frame};
use crate::ipc::Ipc;
use crate::options::{Options, OutputKind};
use crate::{output, source};

pub(crate) fn run(
    options: Options,
    once: bool,
    ipc_mode: bool,
    configured: bool,
) -> Result<(), String> {
    let ipc = if ipc_mode {
        Some(Ipc::start(options.clone())?)
    } else {
        None
    };
    if let Some(ipc) = &ipc {
        if options.output == OutputKind::Json {
            output::ipc::notify("ready", &ipc.address)?;
        }
    }
    let output = if ipc_mode {
        output::create_ipc(&options)
    } else {
        output::create(
            options.output,
            options.inactive_opacity,
            options.waybar_signal,
        )
    };
    let mut runtime = Runtime {
        options: options.clone(),
        engine: None,
        output,
        ipc_mode,
        next_resync: Instant::now(),
        failed_at: None,
        last_frame: None,
    };
    if configured {
        runtime.configure(options, !once)?;
    }
    if once {
        runtime.engine.as_mut().unwrap().refresh()?;
        return runtime.emit();
    }
    loop {
        if let Some(ipc) = &ipc {
            while let Ok(request) = ipc.requests.try_recv() {
                let _ = request.reply.send(runtime.configure(request.options, true));
            }
        }
        runtime.tick()?;
        thread::sleep(Duration::from_millis(16));
    }
}

struct Runtime {
    options: Options,
    engine: Option<Engine>,
    output: Box<dyn output::Output>,
    ipc_mode: bool,
    next_resync: Instant,
    failed_at: Option<Instant>,
    last_frame: Option<Frame>,
}

impl Runtime {
    fn configure(&mut self, options: Options, watch: bool) -> Result<(), String> {
        let source_changed = self.options.source != options.source
            || self.options.source_endpoint != options.source_endpoint;
        if let Some(engine) = self.engine.as_mut().filter(|_| !source_changed) {
            if self.options.cover_dir != options.cover_dir
                || self.options.current_cover != options.current_cover
            {
                engine.set_cover_cache(options.cover_dir.clone(), options.current_cover.clone());
            }
        } else {
            let source = source::create(options.source, &options.source_endpoint, watch)?;
            self.engine = Some(Engine::new(
                source,
                options.cover_dir.clone(),
                options.current_cover.clone(),
            ));
            self.next_resync = Instant::now();
            self.failed_at = None;
        }
        if self.options.output != options.output
            || self.options.inactive_opacity != options.inactive_opacity
            || self.options.waybar_signal != options.waybar_signal
        {
            self.output = if self.ipc_mode {
                output::create_ipc(&options)
            } else {
                output::create(
                    options.output,
                    options.inactive_opacity,
                    options.waybar_signal,
                )
            };
            self.last_frame = None;
        }
        self.options = options;
        Ok(())
    }

    fn tick(&mut self) -> Result<(), String> {
        let Some(engine) = &mut self.engine else {
            return Ok(());
        };
        let now = Instant::now();
        let refresh = match engine.take_source_update() {
            Ok(changed) if changed || now >= self.next_resync => {
                self.next_resync = now + Duration::from_secs(1);
                Some(engine.refresh())
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        };
        if let Some(result) = refresh {
            match result {
                Ok(()) => self.failed_at = None,
                Err(_) => {
                    if self.failed_at.get_or_insert(now).elapsed() >= Duration::from_secs(2) {
                        engine.clear_playback();
                    }
                }
            }
        }
        self.emit()
    }

    fn emit(&mut self) -> Result<(), String> {
        let options = &self.options;
        let frame = self.engine.as_mut().unwrap().frame(
            options.offset_ms,
            options.max_chars,
            options.alignment,
            options.subtitle,
        );
        if self.last_frame.as_ref() != Some(&frame) {
            self.output.emit(&frame)?;
            self.last_frame = Some(frame);
        }
        Ok(())
    }
}
