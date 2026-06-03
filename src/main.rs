mod app;
mod cli;
mod connector;
mod error;
mod import;
mod logging;
mod platform;
mod process;
mod profile;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    match logging::init(cli.verbose, false) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Failed to initialize logging: {e}");
        }
    }

    if let Err(e) = app::run(cli.command, cli.verbose).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}