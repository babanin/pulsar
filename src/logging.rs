use std::path::PathBuf;

use tracing_subscriber::{fmt, EnvFilter};

pub fn init(verbose: bool, log_to_file: bool) -> Result<(), Box<dyn std::error::Error>> {
    let level = if verbose { "debug" } else { "info" };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    if log_to_file {
        let log_dir = log_dir()?;
        std::fs::create_dir_all(&log_dir)?;
        let log_file = log_dir.join("pulsar.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        fmt()
            .with_env_filter(env_filter)
            .with_writer(file)
            .with_ansi(false)
            .init();
    } else {
        fmt()
            .with_env_filter(env_filter)
            .init();
    }

    Ok(())
}

pub fn log_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = dirs::config_dir()
        .ok_or("Cannot determine config directory")?
        .join("pulsar")
        .join("logs");
    Ok(dir)
}