use std::path::PathBuf;

use serde::Serialize;

use crate::cli::{Commands, ProfileCommands};
use crate::connector::{create_connector, ConnectorConfig};
use crate::error::PulsarError;
use crate::import::{self, ImportSource};
use crate::output;
use crate::platform::{self, Platform};
use crate::profile::model::Profile;
use crate::profile::store::ProfileStore;

pub async fn run(command: Commands, _verbose: bool, json: bool) -> crate::error::Result<()> {
    match command {
        Commands::Doctor => doctor(json).await,
        Commands::Profile(cmd) => profile_cmd(cmd, json).await,
        Commands::Connect {
            name,
            use_system_binaries,
        } => connect(&name, use_system_binaries, json).await,
        Commands::Disconnect => disconnect(json).await,
        Commands::Status => status(json).await,
    }
}

async fn doctor(json: bool) -> crate::error::Result<()> {
    #[derive(Serialize)]
    struct Check {
        name: String,
        ok: bool,
        message: String,
    }

    let mut checks: Vec<Check> = Vec::new();

    match Platform::current() {
        Ok(platform) => checks.push(Check {
            name: "Platform".into(),
            ok: true,
            message: platform.dir_name().to_string(),
        }),
        Err(e) => checks.push(Check {
            name: "Platform".into(),
            ok: false,
            message: format!("Platform not supported: {e}"),
        }),
    }

    if let Ok(plat) = Platform::current() {
        match platform::resolve_binaries(plat) {
            Ok(paths) => {
                checks.push(Check {
                    name: "ck-client".into(),
                    ok: true,
                    message: paths.ck_client.display().to_string(),
                });

                match platform::check_binary_executable(&paths.ck_client) {
                    Ok(()) => checks.push(Check {
                        name: "ck-client executable".into(),
                        ok: true,
                        message: "yes".into(),
                    }),
                    Err(e) => checks.push(Check {
                        name: "ck-client executable".into(),
                        ok: false,
                        message: e.to_string(),
                    }),
                }

                checks.push(Check {
                    name: "openvpn".into(),
                    ok: true,
                    message: paths.openvpn.display().to_string(),
                });

                match platform::check_binary_executable(&paths.openvpn) {
                    Ok(()) => checks.push(Check {
                        name: "openvpn executable".into(),
                        ok: true,
                        message: "yes".into(),
                    }),
                    Err(e) => checks.push(Check {
                        name: "openvpn executable".into(),
                        ok: false,
                        message: e.to_string(),
                    }),
                }
            }
            Err(e) => checks.push(Check {
                name: "Bundled binaries".into(),
                ok: false,
                message: e.to_string(),
            }),
        }
    }

    let store = ProfileStore::new()?;
    let config_dir = store.base_dir().to_path_buf();
    match std::fs::metadata(&config_dir) {
        Ok(_) => checks.push(Check {
            name: "Config directory".into(),
            ok: true,
            message: config_dir.display().to_string(),
        }),
        Err(_) => checks.push(Check {
            name: "Config directory".into(),
            ok: false,
            message: format!("not writable: {}", config_dir.display()),
        }),
    }

    let all_ok = checks.iter().all(|c| c.ok);

    if json {
        #[derive(Serialize)]
        struct DoctorData {
            all_ok: bool,
            checks: Vec<Check>,
        }
        output::result(&DoctorData { all_ok, checks });
    } else {
        for c in &checks {
            let mark = if c.ok { "✓" } else { "✗" };
            println!("{mark} {}: {}", c.name, c.message);
        }
        println!();
        if all_ok {
            println!("All checks passed.");
        } else {
            println!("Some checks failed.");
        }
    }

    Ok(())
}

async fn profile_cmd(cmd: ProfileCommands, json: bool) -> crate::error::Result<()> {
    match cmd {
        ProfileCommands::ImportAmnezia { name, file } => import_amnezia(&name, &file, json).await,
        ProfileCommands::Import { name, ovpn, cloak } => {
            import_manual(&name, &ovpn, &cloak, json).await
        }
        ProfileCommands::List => list_profiles(json).await,
        ProfileCommands::Show { name } => show_profile(&name, json).await,
    }
}

async fn import_amnezia(name: &str, file: &str, json: bool) -> crate::error::Result<()> {
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

    if json {
        #[derive(Serialize)]
        struct ImportResult {
            name: String,
        }
        output::result(&ImportResult { name });
    } else {
        println!("Profile '{}' imported successfully.", name);
    }

    Ok(())
}

async fn import_manual(
    name: &str,
    ovpn_path: &str,
    cloak_path: &str,
    json: bool,
) -> crate::error::Result<()> {
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

    if json {
        #[derive(Serialize)]
        struct ImportResult {
            name: String,
        }
        output::result(&ImportResult { name });
    } else {
        println!("Profile '{}' imported successfully.", name);
    }

    Ok(())
}

async fn list_profiles(json: bool) -> crate::error::Result<()> {
    let store = ProfileStore::new()?;
    let profiles = store.list()?;

    if json {
        #[derive(Serialize)]
        struct ProfileEntry {
            name: String,
            protocol: String,
            remote_host: String,
            remote_port: u16,
            local_port: u16,
        }

        let entries: Vec<ProfileEntry> = profiles
            .iter()
            .map(|p| ProfileEntry {
                name: p.name.clone(),
                protocol: p.protocol_type.to_string(),
                remote_host: p.remote_host.clone(),
                remote_port: p.remote_port,
                local_port: p.local_cloak_port,
            })
            .collect();
        output::result(&entries);
    } else {
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
    }

    Ok(())
}

async fn show_profile(name: &str, json: bool) -> crate::error::Result<()> {
    let store = ProfileStore::new()?;
    let profile = store.load(name)?;

    if json {
        output::result(&profile);
    } else {
        println!("Profile:     {}", profile.name);
        println!("Protocol:    {}", profile.protocol_type);
        println!("Remote host: {}", profile.remote_host);
        println!("Remote port: {}", profile.remote_port);
        println!("Local port:  {}", profile.local_cloak_port);
        println!("Created:     {}", profile.created_at);
    }

    Ok(())
}

async fn connect(name: &str, use_system_binaries: bool, json: bool) -> crate::error::Result<()> {
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

    if json {
        #[derive(Serialize)]
        struct ConnectResult {
            profile: String,
            connected: bool,
        }
        output::result(&ConnectResult {
            profile: name.to_string(),
            connected: true,
        });
    } else {
        println!("Connected to '{}'.", name);
    }

    Ok(())
}

async fn disconnect(json: bool) -> crate::error::Result<()> {
    if json {
        #[derive(Serialize)]
        struct DisconnectResult {
            disconnected: bool,
        }
        output::result(&DisconnectResult { disconnected: true });
    } else {
        println!("Disconnect: no active session found.");
    }
    Ok(())
}

async fn status(json: bool) -> crate::error::Result<()> {
    if json {
        #[derive(Serialize)]
        struct StatusResult {
            connected: bool,
        }
        output::result(&StatusResult { connected: false });
    } else {
        println!("Disconnected");
    }
    Ok(())
}
