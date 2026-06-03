use serde::Deserialize;

use crate::error::{PulsarError, Result};
use crate::profile::model::{CloakConfig, Profile, ProfileData, ProtocolType};

#[derive(Debug, Deserialize)]
struct AmneziaBackup {
    #[serde(rename = "Servers/serversList")]
    servers_list: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AmneziaServer {
    containers: Vec<AmneziaContainer>,
}

#[derive(Debug, Deserialize)]
struct AmneziaContainer {
    container: Option<String>,
    cloak: Option<AmneziaCloakEntry>,
    openvpn: Option<AmneziaOpenVpnEntry>,
}

#[derive(Debug, Deserialize)]
struct AmneziaCloakEntry {
    #[serde(rename = "last_config")]
    last_config: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AmneziaOpenVpnEntry {
    #[serde(rename = "last_config")]
    last_config: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenVpnLastConfig {
    #[serde(rename = "config")]
    config: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CloakRawConfig {
    #[serde(rename = "BrowserSig")]
    browser_sig: Option<String>,
    #[serde(rename = "EncryptionMethod")]
    encryption_method: Option<String>,
    #[serde(rename = "NumConn")]
    num_conn: Option<u32>,
    #[serde(rename = "ProxyMethod")]
    proxy_method: Option<String>,
    #[serde(rename = "PublicKey")]
    public_key: Option<String>,
    #[serde(rename = "RemoteHost")]
    remote_host: Option<String>,
    #[serde(rename = "RemotePort")]
    remote_port: Option<String>,
    #[serde(rename = "ServerName")]
    server_name: Option<String>,
    #[serde(rename = "StreamTimeout")]
    stream_timeout: Option<u32>,
    #[serde(rename = "Transport")]
    transport: Option<String>,
    #[serde(rename = "UID")]
    uid: Option<String>,
}

impl From<CloakRawConfig> for CloakConfig {
    fn from(raw: CloakRawConfig) -> Self {
        CloakConfig {
            browser_sig: raw.browser_sig.unwrap_or_default(),
            encryption_method: raw.encryption_method.unwrap_or_default(),
            num_conn: raw.num_conn.unwrap_or(1),
            proxy_method: raw.proxy_method.unwrap_or_default(),
            public_key: raw.public_key.unwrap_or_default(),
            remote_host: raw.remote_host.unwrap_or_default(),
            remote_port: raw.remote_port.unwrap_or_default(),
            server_name: raw.server_name.unwrap_or_default(),
            stream_timeout: raw.stream_timeout.unwrap_or(300),
            transport: raw.transport.unwrap_or_default(),
            uid: raw.uid.unwrap_or_default(),
        }
    }
}

fn parse_cloak_config(raw_json: &str) -> Result<CloakConfig> {
    let raw: CloakRawConfig =
        serde_json::from_str(raw_json).map_err(|e| {
            PulsarError::InvalidCloakConfig(format!("Invalid JSON: {e}"))
        })?;
    let config = CloakConfig::from(raw);
    config.validate()?;
    Ok(config)
}

fn parse_openvpn_config(raw_json: &str) -> Result<String> {
    let parsed: OpenVpnLastConfig =
        serde_json::from_str(raw_json).map_err(|e| {
            PulsarError::InvalidAmneziaProfile(format!(
                "Invalid OpenVPN config JSON: {e}"
            ))
        })?;
    parsed.config.ok_or(PulsarError::MissingOpenVpnConfig)
}

fn extract_remote_info(openvpn_config: &str) -> (String, u16) {
    let remote_re = regex::Regex::new(r"(?m)^remote\s+(\S+)\s+(\d+)").unwrap();
    if let Some(caps) = remote_re.captures(openvpn_config) {
        let host = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let port: u16 = caps
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(1194);
        (host, port)
    } else {
        ("127.0.0.1".to_string(), 1194)
    }
}

pub fn import_amnezia_backup(contents: &str) -> Result<ProfileData> {
    let backup: AmneziaBackup = serde_json::from_str(contents).map_err(|e| {
        PulsarError::InvalidAmneziaProfile(format!("Invalid backup JSON: {e}"))
    })?;

    let servers_json = backup
        .servers_list
        .ok_or_else(|| {
            PulsarError::InvalidAmneziaProfile(
                "Missing Servers/serversList".to_string(),
            )
        })?;

    let servers: Vec<AmneziaServer> =
        serde_json::from_str(&servers_json).map_err(|e| {
            PulsarError::InvalidAmneziaProfile(format!(
                "Invalid servers list JSON: {e}"
            ))
        })?;

    for server in &servers {
        for container in &server.containers {
            if container.container.as_deref() == Some("amnezia-openvpn-cloak") {
                if let (Some(cloak_entry), Some(openvpn_entry)) =
                    (&container.cloak, &container.openvpn)
                {
                    return extract_openvpn_cloak(
                        cloak_entry,
                        openvpn_entry,
                    );
                }
            }
        }
    }

    Err(PulsarError::InvalidAmneziaProfile(
        "No amnezia-openvpn-cloak container found".to_string(),
    ))
}

fn extract_openvpn_cloak(
    cloak_entry: &AmneziaCloakEntry,
    openvpn_entry: &AmneziaOpenVpnEntry,
) -> Result<ProfileData> {
    let cloak_raw = cloak_entry
        .last_config
        .as_deref()
        .ok_or_else(|| {
            PulsarError::InvalidCloakConfig(
                "Missing cloak last_config".to_string(),
            )
        })?;
    let cloak_config = parse_cloak_config(cloak_raw)?;

    let openvpn_raw = openvpn_entry
        .last_config
        .as_deref()
        .ok_or_else(|| {
            PulsarError::InvalidAmneziaProfile(
                "Missing OpenVPN last_config".to_string(),
            )
        })?;
    let openvpn_config = parse_openvpn_config(openvpn_raw)?;

    let (remote_host, remote_port) = extract_remote_info(&openvpn_config);
    let is_local = remote_host == "127.0.0.1";

    let profile = Profile {
        name: String::new(),
        protocol_type: ProtocolType::OpenvpnCloak,
        created_at: chrono::Utc::now().to_rfc3339(),
        remote_host: if is_local {
            cloak_config.remote_host.clone()
        } else {
            remote_host
        },
        remote_port: if is_local {
            cloak_config
                .remote_port
                .parse()
                .unwrap_or(remote_port)
        } else {
            remote_port
        },
        local_cloak_port: 1194,
    };

    Ok(ProfileData {
        profile,
        openvpn_config,
        cloak_config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cloak_config_valid() {
        let json = r#"{
            "BrowserSig": "chrome",
            "EncryptionMethod": "aes-gcm",
            "NumConn": 1,
            "ProxyMethod": "openvpn",
            "PublicKey": "dGVzdA==",
            "RemoteHost": "1.2.3.4",
            "RemotePort": "443",
            "ServerName": "example.com",
            "StreamTimeout": 300,
            "Transport": "direct",
            "UID": "dGVzdHVpZA=="
        }"#;
        let config = parse_cloak_config(json).unwrap();
        assert_eq!(config.public_key, "dGVzdA==");
        assert_eq!(config.proxy_method, "openvpn");
    }

    #[test]
    fn test_parse_cloak_config_missing_required() {
        let json = r#"{
            "BrowserSig": "chrome"
        }"#;
        let result = parse_cloak_config(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_remote_info() {
        let config = "client\ndev tun\nremote 10.0.0.1 443\n";
        let (host, port) = extract_remote_info(config);
        assert_eq!(host, "10.0.0.1");
        assert_eq!(port, 443);
    }

    #[test]
    fn test_extract_remote_info_defaults() {
        let config = "client\ndev tun\n";
        let (host, port) = extract_remote_info(config);
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 1194);
    }

    #[test]
    fn test_parse_openvpn_config() {
        let json = r#"{"config": "client\ndev tun", "clientId": "test"}"#;
        let config = parse_openvpn_config(json).unwrap();
        assert!(config.contains("client"));
    }
}