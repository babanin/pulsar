pub mod openvpn_cloak;

use std::fmt;

use async_trait::async_trait;

use crate::error::Result;
use crate::profile::model::ProtocolType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting"),
            ConnectionStatus::Connected => write!(f, "Connected"),
        }
    }
}

pub struct ConnectorConfig {
    pub profile_name: String,
    pub openvpn_config_path: String,
    pub cloak_config_path: String,
    pub ck_client_path: String,
    pub openvpn_bin_path: String,
    pub local_cloak_port: u16,
    pub use_system_binaries: bool,
}

#[async_trait]
#[allow(dead_code)]
pub trait Connector: Send + Sync {
    fn protocol_type(&self) -> ProtocolType;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    fn status(&self) -> ConnectionStatus;
}

pub fn create_connector(
    protocol_type: ProtocolType,
    config: ConnectorConfig,
) -> Box<dyn Connector> {
    match protocol_type {
        ProtocolType::OpenvpnCloak => Box::new(openvpn_cloak::OpenVpnCloakConnector::new(
            config.profile_name,
            config.openvpn_config_path,
            config.cloak_config_path,
            config.ck_client_path,
            config.openvpn_bin_path,
            config.local_cloak_port,
            config.use_system_binaries,
        )),
    }
}
