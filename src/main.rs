mod cli;
mod cover;
mod engine;
mod ipc;
mod lyrics;
mod model;
mod options;
mod output;
mod runtime;
mod source;

fn run() -> Result<(), String> {
    if std::env::args_os().len() > 1 {
        let launch = cli::parse()?;
        if let Some(command) = launch.control {
            return source::control(
                launch.options.source,
                &launch.options.source_endpoint,
                command,
            );
        }
        return runtime::run(launch.options, launch.once, false, true);
    }
    let bootstrap = ipc::bootstrap()?;
    let configured = bootstrap.is_some();
    runtime::run(bootstrap.unwrap_or_default(), false, true, configured)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("bar-lyrics: {error}");
        std::process::exit(1);
    }
}
