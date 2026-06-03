use std::path::PathBuf;

use crate::error::{PulsarError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOSX86_64,
    MacOSAarch64,
    LinuxX86_64,
}

impl Platform {
    pub fn current() -> Result<Self> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        match (os, arch) {
            ("macos", "x86_64") => Ok(Platform::MacOSX86_64),
            ("macos", "aarch64") => Ok(Platform::MacOSAarch64),
            ("linux", "x86_64") => Ok(Platform::LinuxX86_64),
            _ => Err(PulsarError::PlatformNotSupported(format!(
                "{os} {arch}"
            ))),
        }
    }

    pub fn dir_name(&self) -> &'static str {
        match self {
            Platform::MacOSX86_64 => "macos-x86_64",
            Platform::MacOSAarch64 => "macos-aarch64",
            Platform::LinuxX86_64 => "linux-x86_64",
        }
    }

    #[allow(dead_code)]
    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            Platform::MacOSX86_64 | Platform::MacOSAarch64 | Platform::LinuxX86_64
        )
    }
}

pub struct BinaryPaths {
    pub ck_client: PathBuf,
    pub openvpn: PathBuf,
}

pub fn resolve_binaries(platform: Platform) -> Result<BinaryPaths> {
    let exe_dir = std::env::current_exe().map_err(|e| {
        PulsarError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Cannot determine executable path: {e}"),
        ))
    })?;

    let exe_parent = exe_dir
        .parent()
        .ok_or_else(|| {
            PulsarError::BinaryMissing(
                "Cannot determine executable directory".to_string(),
            )
        })?;

    let bundled_dir = exe_parent.join("bundled").join(platform.dir_name());

    let ck_client = bundled_dir.join("ck-client");
    let openvpn = bundled_dir.join("openvpn");

    if !ck_client.exists() {
        return Err(PulsarError::BinaryMissing(
            ck_client.display().to_string(),
        ));
    }
    if !openvpn.exists() {
        return Err(PulsarError::BinaryMissing(
            openvpn.display().to_string(),
        ));
    }

    Ok(BinaryPaths {
        ck_client,
        openvpn,
    })
}

pub fn check_binary_executable(path: &PathBuf) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path).map_err(PulsarError::Io)?;
        let mode = metadata.permissions().mode();
        if mode & 0o111 == 0 {
            return Err(PulsarError::BinaryNotExecutable(
                path.display().to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_dir_names() {
        assert_eq!(Platform::MacOSX86_64.dir_name(), "macos-x86_64");
        assert_eq!(Platform::MacOSAarch64.dir_name(), "macos-aarch64");
        assert_eq!(Platform::LinuxX86_64.dir_name(), "linux-x86_64");
    }

    #[test]
    fn test_supported_platforms() {
        assert!(Platform::MacOSX86_64.is_supported());
        assert!(Platform::MacOSAarch64.is_supported());
        assert!(Platform::LinuxX86_64.is_supported());
    }
}