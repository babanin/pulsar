use crate::error::{PulsarError, Result};
use crate::profile::model::{CloakConfig, Profile, ProfileData, ProtocolType};

pub fn import_manual(ovpn_path: &str, cloak_path: &str) -> Result<ProfileData> {
    let openvpn_config = std::fs::read_to_string(ovpn_path).map_err(|e| {
        PulsarError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot read OpenVPN config from {ovpn_path}: {e}"),
        ))
    })?;

    let cloak_json = std::fs::read_to_string(cloak_path).map_err(|e| {
        PulsarError::Io(std::io::Error::new(
            e.kind(),
            format!("Cannot read Cloak config from {cloak_path}: {e}"),
        ))
    })?;

    let cloak_config: CloakConfig = serde_json::from_str(&cloak_json)
        .map_err(|e| PulsarError::InvalidCloakConfig(format!("Invalid Cloak JSON: {e}")))?;

    cloak_config.validate()?;

    let remote_re = regex::Regex::new(r"(?m)^remote\s+(\S+)\s+(\d+)").unwrap();
    let (remote_host, remote_port) = if let Some(caps) = remote_re.captures(&openvpn_config) {
        let host = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| cloak_config.remote_host.clone());
        let port: u16 = caps
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(443);
        (host, port)
    } else {
        (cloak_config.remote_host.clone(), 443)
    };

    let profile = Profile {
        name: String::new(),
        protocol_type: ProtocolType::OpenvpnCloak,
        created_at: chrono::Utc::now().to_rfc3339(),
        remote_host: if remote_host == "127.0.0.1" {
            cloak_config.remote_host.clone()
        } else {
            remote_host
        },
        remote_port: cloak_config.remote_port.parse().unwrap_or(remote_port),
        local_cloak_port: 1194,
    };

    Ok(ProfileData {
        profile,
        openvpn_config,
        cloak_config,
    })
}
