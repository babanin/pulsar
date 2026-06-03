use serde::{Deserialize, Serialize};

use crate::error::PulsarError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProtocolType {
    OpenvpnCloak,
}

impl std::fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolType::OpenvpnCloak => write!(f, "openvpn-cloak"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CloakConfig {
    pub browser_sig: String,
    pub encryption_method: String,
    pub num_conn: u32,
    pub proxy_method: String,
    pub public_key: String,
    pub remote_host: String,
    pub remote_port: String,
    pub server_name: String,
    pub stream_timeout: u32,
    pub transport: String,
    #[serde(rename = "UID")]
    pub uid: String,
}

impl CloakConfig {
    pub fn validate(&self) -> std::result::Result<(), PulsarError> {
        let mut missing = Vec::new();
        if self.public_key.is_empty() {
            missing.push("PublicKey");
        }
        if self.uid.is_empty() {
            missing.push("UID");
        }
        if self.remote_host.is_empty() {
            missing.push("RemoteHost");
        }
        if self.remote_port.is_empty() {
            missing.push("RemotePort");
        }
        if self.proxy_method.is_empty() {
            missing.push("ProxyMethod");
        }
        if !missing.is_empty() {
            return Err(PulsarError::InvalidCloakConfig(format!(
                "Missing required fields: {}",
                missing.join(", ")
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub protocol_type: ProtocolType,
    pub created_at: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub local_cloak_port: u16,
}

impl Profile {
    pub fn sanitize_name(name: &str) -> std::result::Result<String, PulsarError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(PulsarError::InvalidProfileName(
                "Profile name cannot be empty".to_string(),
            ));
        }
        let valid = trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
        if !valid {
            return Err(PulsarError::InvalidProfileName(
                "Profile name can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }
        Ok(trimmed.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ProfileData {
    pub profile: Profile,
    pub openvpn_config: String,
    pub cloak_config: CloakConfig,
}