use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pulsar")]
#[command(about = "Lightweight CLI orchestrator for OpenVPN-over-Cloak")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, short, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(long, global = true, help = "Output in JSON format for AI agents")]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Verify environment and bundled binaries")]
    Doctor,

    #[command(subcommand)]
    Profile(ProfileCommands),

    #[command(about = "Connect to a VPN profile")]
    Connect {
        #[arg(help = "Profile name")]
        name: String,

        #[arg(long, help = "Use system binaries instead of bundled ones")]
        use_system_binaries: bool,
    },

    #[command(about = "Disconnect from the current VPN session")]
    Disconnect,

    #[command(about = "Show current connection status")]
    Status,
}

#[derive(Subcommand)]
pub enum ProfileCommands {
    #[command(about = "Import from an AmneziaVPN export file")]
    ImportAmnezia {
        #[arg(long, short, help = "Profile name")]
        name: String,

        #[arg(long, short, help = "Path to AmneziaVPN export file")]
        file: String,
    },

    #[command(about = "Import OpenVPN and Cloak configs manually")]
    Import {
        #[arg(long, short, help = "Profile name")]
        name: String,

        #[arg(long, help = "Path to OpenVPN config file")]
        ovpn: String,

        #[arg(long, help = "Path to Cloak config file")]
        cloak: String,
    },

    #[command(about = "List all stored profiles")]
    List,

    #[command(about = "Show profile details")]
    Show {
        #[arg(help = "Profile name")]
        name: String,
    },
}
