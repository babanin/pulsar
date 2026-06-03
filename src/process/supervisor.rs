use std::process::Stdio;

use tokio::process::{Child, Command};

pub struct ManagedProcess {
    name: String,
    child: Child,
}

impl ManagedProcess {
    pub async fn new(
        name: &str,
        command: &mut Command,
    ) -> std::io::Result<Self> {
        let child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(Self {
            name: name.to_string(),
            child,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    #[allow(dead_code)]
    pub async fn wait(&mut self) -> std::result::Result<(), String> {
        let status = self
            .child
            .wait()
            .await
            .map_err(|e| format!("Failed to wait for {}: {e}", self.name))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "{} exited with code {:?}",
                self.name,
                status.code()
            ))
        }
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        if let Ok(Some(_)) = self.child.try_wait() {
            return;
        }
        let _ = self.child.start_kill();
    }
}

pub struct ProcessSupervisor {
    processes: Vec<ManagedProcess>,
}

impl Default for ProcessSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
        }
    }

    pub fn add(&mut self, process: ManagedProcess) {
        tracing::debug!("Added process: {}", process.name());
        self.processes.push(process);
    }

    pub async fn wait_any(&mut self) {
        if self.processes.is_empty() {
            return;
        }

        for proc in &mut self.processes {
            if let Ok(Some(status)) = proc.child.try_wait() {
                tracing::warn!(
                    "{} already exited with: {:?}",
                    proc.name(),
                    status.code()
                );
                return;
            }
        }

        if let Some(proc) = self.processes.first_mut() {
            match proc.child.wait().await {
                Ok(status) => {
                    tracing::warn!(
                        "{} exited with status: {:?}",
                        proc.name(),
                        status.code()
                    );
                }
                Err(e) => {
                    tracing::error!("Error waiting for {}: {e}", proc.name());
                }
            }
        }
    }

    pub async fn kill_all(&mut self) {
        let process_count = self.processes.len();
        tracing::info!(
            "Shutting down {process_count} process(es)..."
        );

        let processes = std::mem::take(&mut self.processes);
        let processes: Vec<_> = processes.into_iter().rev().collect();

        for mut proc in processes {
            tracing::info!("Stopping {}...", proc.name());
            match proc.child.kill().await {
                Ok(()) => {
                    tracing::debug!("Sent SIGTERM to {}", proc.name());
                    let _ = proc.child.wait().await;
                }
                Err(e) => {
                    tracing::warn!(
                        "Could not kill {}: {e}",
                        proc.name()
                    );
                }
            }
        }
    }
}