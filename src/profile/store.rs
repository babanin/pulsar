use std::fs;
use std::path::PathBuf;

use crate::error::{PulsarError, Result};
use crate::profile::model::{Profile, ProfileData};

pub struct ProfileStore {
    base_dir: PathBuf,
}

impl ProfileStore {
    pub fn new() -> Result<Self> {
        let base_dir = dirs::config_dir()
            .ok_or_else(|| {
                PulsarError::ConfigDirNotWritable("Cannot determine config directory".to_string())
            })?
            .join("pulsar")
            .join("profiles");

        Self::with_base_dir(base_dir)
    }

    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_dir).map_err(|e| {
            PulsarError::ConfigDirNotWritable(format!("Cannot create {}: {e}", base_dir.display()))
        })?;
        Ok(Self { base_dir })
    }

    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    pub fn profile_dir(&self, name: &str) -> PathBuf {
        self.base_dir.join(name)
    }

    pub fn exists(&self, name: &str) -> bool {
        self.profile_dir(name).join("profile.json").exists()
    }

    pub fn list(&self) -> Result<Vec<Profile>> {
        let mut profiles = Vec::new();
        let entries = fs::read_dir(&self.base_dir).map_err(PulsarError::Io)?;

        for entry in entries {
            let entry = entry.map_err(PulsarError::Io)?;
            let path = entry.path();
            if path.is_dir() && path.join("profile.json").exists() {
                let profile: Profile =
                    serde_json::from_str(&fs::read_to_string(path.join("profile.json"))?)
                        .map_err(PulsarError::Json)?;
                profiles.push(profile);
            }
        }

        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(profiles)
    }

    pub fn load(&self, name: &str) -> Result<Profile> {
        let dir = self.profile_dir(name);
        let profile_path = dir.join("profile.json");
        if !profile_path.exists() {
            return Err(PulsarError::ProfileNotFound(name.to_string()));
        }
        let data = fs::read_to_string(&profile_path).map_err(PulsarError::Io)?;
        let profile: Profile = serde_json::from_str(&data).map_err(PulsarError::Json)?;
        Ok(profile)
    }

    pub fn load_openvpn_config(&self, name: &str) -> Result<String> {
        let dir = self.profile_dir(name);
        let path = dir.join("openvpn.ovpn");
        if !path.exists() {
            return Err(PulsarError::MissingOpenVpnConfig);
        }
        fs::read_to_string(&path).map_err(PulsarError::Io)
    }

    pub fn load_cloak_config(&self, name: &str) -> Result<String> {
        let dir = self.profile_dir(name);
        let path = dir.join("cloak.json");
        if !path.exists() {
            return Err(PulsarError::MissingCloakConfig);
        }
        fs::read_to_string(&path).map_err(PulsarError::Io)
    }

    pub fn save(&self, data: &ProfileData) -> Result<()> {
        let dir = self.profile_dir(&data.profile.name);
        if self.exists(&data.profile.name) {
            return Err(PulsarError::ProfileAlreadyExists(data.profile.name.clone()));
        }

        fs::create_dir_all(&dir).map_err(PulsarError::Io)?;

        let profile_json =
            serde_json::to_string_pretty(&data.profile).map_err(PulsarError::Json)?;
        fs::write(dir.join("profile.json"), profile_json).map_err(PulsarError::Io)?;

        fs::write(dir.join("openvpn.ovpn"), &data.openvpn_config).map_err(PulsarError::Io)?;

        let cloak_json =
            serde_json::to_string_pretty(&data.cloak_config).map_err(PulsarError::Json)?;
        fs::write(dir.join("cloak.json"), cloak_json).map_err(PulsarError::Io)?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete(&self, name: &str) -> Result<()> {
        let dir = self.profile_dir(name);
        if !dir.exists() {
            return Err(PulsarError::ProfileNotFound(name.to_string()));
        }
        fs::remove_dir_all(&dir).map_err(PulsarError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::model::{CloakConfig, ProtocolType};

    fn test_cloak_config() -> CloakConfig {
        CloakConfig {
            browser_sig: "chrome".to_string(),
            encryption_method: "aes-gcm".to_string(),
            num_conn: 1,
            proxy_method: "openvpn".to_string(),
            public_key: "dGVzdA==".to_string(),
            remote_host: "1.2.3.4".to_string(),
            remote_port: "443".to_string(),
            server_name: "example.com".to_string(),
            stream_timeout: 300,
            transport: "direct".to_string(),
            uid: "dGVzdHVpZA==".to_string(),
        }
    }

    #[test]
    fn test_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProfileStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        let profile = Profile {
            name: "test-profile".to_string(),
            protocol_type: ProtocolType::OpenvpnCloak,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            remote_host: "1.2.3.4".to_string(),
            remote_port: 443,
            local_cloak_port: 1194,
        };

        let data = ProfileData {
            profile: profile.clone(),
            openvpn_config: "client\ndev tun".to_string(),
            cloak_config: test_cloak_config(),
        };

        store.save(&data).unwrap();
        let loaded = store.load("test-profile").unwrap();
        assert_eq!(loaded.name, "test-profile");
        assert_eq!(loaded.remote_host, "1.2.3.4");
    }

    #[test]
    fn test_save_duplicate_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProfileStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        let profile = Profile {
            name: "dup".to_string(),
            protocol_type: ProtocolType::OpenvpnCloak,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            remote_host: "1.2.3.4".to_string(),
            remote_port: 443,
            local_cloak_port: 1194,
        };

        let data = ProfileData {
            profile: profile.clone(),
            openvpn_config: "client".to_string(),
            cloak_config: test_cloak_config(),
        };

        store.save(&data).unwrap();
        let result = store.save(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProfileStore::with_base_dir(tmp.path().to_path_buf()).unwrap();
        let result = store.load("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_profiles() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ProfileStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

        for name in &["charlie", "alpha", "bravo"] {
            let profile = Profile {
                name: name.to_string(),
                protocol_type: ProtocolType::OpenvpnCloak,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                remote_host: "1.2.3.4".to_string(),
                remote_port: 443,
                local_cloak_port: 1194,
            };
            let data = ProfileData {
                profile,
                openvpn_config: "client".to_string(),
                cloak_config: test_cloak_config(),
            };
            store.save(&data).unwrap();
        }

        let list = store.list().unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].name, "alpha");
        assert_eq!(list[1].name, "bravo");
        assert_eq!(list[2].name, "charlie");
    }
}
