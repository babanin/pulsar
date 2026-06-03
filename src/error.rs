use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum PulsarError {
#[error("Profile not found: {0}")]
    ProfileNotFound(String),

    #[error("Profile already exists: {0}")]
    ProfileAlreadyExists(String),

    #[error("Invalid profile name: {0}")]
    InvalidProfileName(String),

    #[error("Bundled binary missing: {0}")]
    BinaryMissing(String),

    #[error("Bundled binary not executable: {0}")]
    BinaryNotExecutable(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Invalid Amnezia profile: {0}")]
    InvalidAmneziaProfile(String),

    #[error("Missing Cloak configuration")]
    MissingCloakConfig,

    #[error("Missing OpenVPN configuration")]
    MissingOpenVpnConfig,

    #[error("Invalid Cloak configuration: {0}")]
    InvalidCloakConfig(String),

    #[error("Failed to start {process}: {source}")]
    ProcessStartFailed {
        process: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{process} exited unexpectedly with code {code}")]
    ProcessExited { process: String, code: i32 },

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Not connected")]
    NotConnected,

    #[error("Already connected")]
    AlreadyConnected,

    #[error("Cloak did not become ready at {host}:{port} within {timeout}s")]
    CloakReadyTimeout { host: String, port: u16, timeout: u64 },

    #[error("Configuration directory not writable: {0}")]
    ConfigDirNotWritable(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

pub type Result<T> = std::result::Result<T, PulsarError>;