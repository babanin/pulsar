mod app;
mod cli;
mod connector;
mod error;
mod import;
mod logging;
mod output;
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

    if let Err(e) = app::run(cli.command, cli.verbose, cli.json).await {
        if cli.json {
            output::error(&e.to_string());
        } else {
            eprintln!("Error: {e}");
        }
        std::process::exit(1);
    }

    Ok(())
}
