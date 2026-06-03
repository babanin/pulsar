use std::path::PathBuf;

use crate::cli::{Commands, ProfileCommands};
use crate::connector::{create_connector, ConnectorConfig};
use crate::error::PulsarError;
use crate::import::{self, ImportSource};
use crate::platform::{self, Platform};
use crate::profile::model::Profile;
use crate::profile::store::ProfileStore;

pub async fn run(command: Commands, _verbose: bool) -> crate::error::Result<()> {
    match command {
        Commands::Doctor => doctor().await,
        Commands::Profile(cmd) => profile_cmd(cmd).await,
        Commands::Connect {
            name,
            use_system_binaries,
        } => connect(&name, use_system_binaries).await,
        Commands::Disconnect => disconnect().await,
        Commands::Status => status().await,
    }
}

async fn doctor() -> crate::error::Result<()> {
    let mut all_ok = true;

    match Platform::current() {
        Ok(platform) => {
            println!("✓ Platform supported: {}", platform.dir_name());
        }
        Err(e) => {
            println!("✗ {e}");
            all_ok = false;
        }
    }

    let platform = Platform::current().ok();
    if let Some(plat) = platform {
        match platform::resolve_binaries(plat) {
            Ok(paths) => {
                println!("✓ ck-client found: {}", paths.ck_client.display());
                if let Err(e) = platform::check_binary_executable(&paths.ck_client) {
                    println!("✗ ck-client not executable: {e}");
                    all_ok = false;
                } else {
                    println!("✓ ck-client executable");
                }

                println!("✓ openvpn found: {}", paths.openvpn.display());
                if let Err(e) = platform::check_binary_executable(&paths.openvpn) {
                    println!("✗ openvpn not executable: {e}");
                    all_ok = false;
                } else {
                    println!("✓ openvpn executable");
                }
            }
            Err(e) => {
                println!("✗ {e}");
                all_ok = false;
            }
        }
    }

    let store = ProfileStore::new()?;
    let config_dir = store.base_dir();
    match std::fs::metadata(config_dir) {
        Ok(_) => {
            println!(
                "✓ Configuration directory writable: {}",
                config_dir.display()
            );
        }
        Err(_) => {
            println!(
                "✗ Configuration directory not writable: {}",
                config_dir.display()
            );
            all_ok = false;
        }
    }

    if all_ok {
        println!("\nAll checks passed.");
    } else {
        println!("\nSome checks failed.");
    }

    Ok(())
}

async fn profile_cmd(cmd: ProfileCommands) -> crate::error::Result<()> {
    match cmd {
        ProfileCommands::ImportAmnezia { name, file } => import_amnezia(&name, &file).await,
        ProfileCommands::Import { name, ovpn, cloak } => import_manual(&name, &ovpn, &cloak).await,
        ProfileCommands::List => list_profiles().await,
        ProfileCommands::Show { name } => show_profile(&name).await,
    }
}

async fn import_amnezia(name: &str, file: &str) -> crate::error::Result<()> {
    let name = Profile::sanitize_name(name)?;
    let store = ProfileStore::new()?;

    if store.exists(&name) {
        return Err(PulsarError::ProfileAlreadyExists(name));
    }

    let mut data = import::import(ImportSource::AmneziaBackup {
        path: file.to_string(),
    })?;
    data.profile.name = name.clone();

    store.save(&data)?;
    println!("Profile '{}' imported successfully.", name);
    Ok(())
}

async fn import_manual(name: &str, ovpn_path: &str, cloak_path: &str) -> crate::error::Result<()> {
    let name = Profile::sanitize_name(name)?;
    let store = ProfileStore::new()?;

    if store.exists(&name) {
        return Err(PulsarError::ProfileAlreadyExists(name));
    }

    let mut data = import::import(ImportSource::Manual {
        ovpn_path: ovpn_path.to_string(),
        cloak_path: cloak_path.to_string(),
    })?;
    data.profile.name = name.clone();

    store.save(&data)?;
    println!("Profile '{}' imported successfully.", name);
    Ok(())
}

async fn list_profiles() -> crate::error::Result<()> {
    let store = ProfileStore::new()?;
    let profiles = store.list()?;

    if profiles.is_empty() {
        println!("No profiles found.");
        return Ok(());
    }

    println!("{:<20} {:<15} {:<10} HOST", "NAME", "PROTOCOL", "PORT");
    for p in &profiles {
        println!(
            "{:<20} {:<15} {:<10} {}",
            p.name, p.protocol_type, p.remote_port, p.remote_host
        );
    }

    Ok(())
}

async fn show_profile(name: &str) -> crate::error::Result<()> {
    let store = ProfileStore::new()?;
    let profile = store.load(name)?;

    println!("Profile:     {}", profile.name);
    println!("Protocol:    {}", profile.protocol_type);
    println!("Remote host: {}", profile.remote_host);
    println!("Remote port: {}", profile.remote_port);
    println!("Local port:  {}", profile.local_cloak_port);
    println!("Created:     {}", profile.created_at);

    Ok(())
}

async fn connect(name: &str, use_system_binaries: bool) -> crate::error::Result<()> {
    let store = ProfileStore::new()?;
    let profile = store.load(name)?;
    let _openvpn_config = store.load_openvpn_config(name)?;
    let _cloak_config = store.load_cloak_config(name)?;

    let platform = Platform::current()?;
    let binaries = if use_system_binaries {
        platform::BinaryPaths {
            ck_client: PathBuf::from("ck-client"),
            openvpn: PathBuf::from("openvpn"),
        }
    } else {
        platform::resolve_binaries(platform)?
    };

    let profile_dir = store.profile_dir(name);
    let ovpn_path = profile_dir.join("openvpn.ovpn");
    let cloak_path = profile_dir.join("cloak.json");

    let conn = create_connector(
        profile.protocol_type,
        ConnectorConfig {
            profile_name: profile.name.clone(),
            openvpn_config_path: ovpn_path.to_string_lossy().to_string(),
            cloak_config_path: cloak_path.to_string_lossy().to_string(),
            ck_client_path: binaries.ck_client.to_string_lossy().to_string(),
            openvpn_bin_path: binaries.openvpn.to_string_lossy().to_string(),
            local_cloak_port: profile.local_cloak_port,
            use_system_binaries,
        },
    );

    tracing::info!("Connecting to '{}'...", profile.name);

    conn.start().await?;

    Ok(())
}

async fn disconnect() -> crate::error::Result<()> {
    println!("Disconnect: no active session found.");
    Ok(())
}

async fn status() -> crate::error::Result<()> {
    println!("Disconnected");
    Ok(())
}
