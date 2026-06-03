use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::connector::{ConnectionStatus, Connector};
use crate::error::{PulsarError, Result};
use crate::process::supervisor::{ManagedProcess, ProcessSupervisor};
use crate::profile::model::ProtocolType;

const CLOAK_READY_TIMEOUT_SECS: u64 = 30;
const CLOAK_POLL_INTERVAL_MS: u64 = 100;

pub struct OpenVpnCloakConnector {
    profile_name: String,
    openvpn_config_path: PathBuf,
    cloak_config_path: PathBuf,
    ck_client_path: PathBuf,
    openvpn_bin_path: PathBuf,
    local_cloak_port: u16,
    use_system_binaries: bool,
    supervisor: Arc<Mutex<ProcessSupervisor>>,
    status: Arc<AtomicI32>,
}

impl OpenVpnCloakConnector {
    pub fn new(
        profile_name: String,
        openvpn_config_path: String,
        cloak_config_path: String,
        ck_client_path: String,
        openvpn_bin_path: String,
        local_cloak_port: u16,
        use_system_binaries: bool,
    ) -> Self {
        Self {
            profile_name,
            openvpn_config_path: PathBuf::from(openvpn_config_path),
            cloak_config_path: PathBuf::from(cloak_config_path),
            ck_client_path: PathBuf::from(ck_client_path),
            openvpn_bin_path: PathBuf::from(openvpn_bin_path),
            local_cloak_port,
            use_system_binaries,
            supervisor: Arc::new(Mutex::new(ProcessSupervisor::new())),
            status: Arc::new(AtomicI32::new(0)),
        }
    }

    async fn wait_for_cloak_ready(&self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.local_cloak_port);
        let timeout = Duration::from_secs(CLOAK_READY_TIMEOUT_SECS);
        let start = tokio::time::Instant::now();

        loop {
            match TcpStream::connect(&addr).await {
                Ok(_) => {
                    tracing::info!("Cloak is ready at {}", addr);
                    return Ok(());
                }
                Err(_) => {
                    if start.elapsed() > timeout {
                        return Err(PulsarError::CloakReadyTimeout {
                            host: "127.0.0.1".to_string(),
                            port: self.local_cloak_port,
                            timeout: CLOAK_READY_TIMEOUT_SECS,
                        });
                    }
                    tokio::time::sleep(Duration::from_millis(CLOAK_POLL_INTERVAL_MS)).await;
                }
            }
        }
    }
}

const STATUS_DISCONNECTED: i32 = 0;
const STATUS_CONNECTING: i32 = 1;
const STATUS_CONNECTED: i32 = 2;

#[async_trait]
impl Connector for OpenVpnCloakConnector {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::OpenvpnCloak
    }

    async fn start(&self) -> Result<()> {
        self.status.store(STATUS_CONNECTING, Ordering::SeqCst);
        tracing::info!(
            "Starting OpenVPN-over-Cloak connection for profile '{}'",
            self.profile_name
        );

        let ck_path = if self.use_system_binaries {
            "ck-client"
        } else {
            self.ck_client_path.to_str().ok_or_else(|| {
                PulsarError::BinaryMissing("ck-client path is invalid".to_string())
            })?
        };

        let ovpn_path = if self.use_system_binaries {
            "openvpn"
        } else {
            self.openvpn_bin_path
                .to_str()
                .ok_or_else(|| PulsarError::BinaryMissing("openvpn path is invalid".to_string()))?
        };

        let mut supervisor = self.supervisor.lock().await;

        tracing::info!(
            "Starting Cloak: {} -c {} -l {}",
            ck_path,
            self.cloak_config_path.display(),
            self.local_cloak_port
        );

        let cloak_process = ManagedProcess::new(
            "cloak",
            Command::new(ck_path)
                .arg("-c")
                .arg(&self.cloak_config_path)
                .arg("-l")
                .arg(self.local_cloak_port.to_string()),
        )
        .await
        .map_err(|e| PulsarError::ProcessStartFailed {
            process: "ck-client".to_string(),
            source: e,
        })?;

        supervisor.add(cloak_process);

        drop(supervisor);
        self.wait_for_cloak_ready().await?;

        tracing::info!(
            "Starting OpenVPN: {} --config {}",
            ovpn_path,
            self.openvpn_config_path.display()
        );

        let mut supervisor = self.supervisor.lock().await;

        let openvpn_process = ManagedProcess::new(
            "openvpn",
            Command::new(ovpn_path)
                .arg("--config")
                .arg(&self.openvpn_config_path),
        )
        .await
        .map_err(|e| PulsarError::ProcessStartFailed {
            process: "openvpn".to_string(),
            source: e,
        })?;

        supervisor.add(openvpn_process);
        self.status.store(STATUS_CONNECTED, Ordering::SeqCst);

        tracing::info!("Connection established for profile '{}'", self.profile_name);

        supervisor.wait_any().await;

        self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
        supervisor.kill_all().await;

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping connection for profile '{}'", self.profile_name);
        let mut supervisor = self.supervisor.lock().await;
        supervisor.kill_all().await;
        self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
        Ok(())
    }

    fn status(&self) -> ConnectionStatus {
        match self.status.load(Ordering::SeqCst) {
            0 => ConnectionStatus::Disconnected,
            1 => ConnectionStatus::Connecting,
            2 => ConnectionStatus::Connected,
            _ => ConnectionStatus::Disconnected,
        }
    }
}
