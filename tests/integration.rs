use pulsar::import::{self, ImportSource};
use pulsar::profile::model::{CloakConfig, Profile, ProtocolType};
use pulsar::profile::store::ProfileStore;

fn fixtures_dir() -> String {
    let mut dir = std::env::current_dir().unwrap();
    dir.push("tests");
    dir.push("fixtures");
    dir.to_string_lossy().to_string()
}

#[test]
fn test_import_amnezia_backup() {
    let backup_path = format!("{}/amnezia-openvpn-cloak.backup", fixtures_dir());
    let data = import::import(ImportSource::AmneziaBackup {
        path: backup_path,
    })
    .unwrap();

    assert_eq!(data.profile.protocol_type, ProtocolType::OpenvpnCloak);
    assert_eq!(data.cloak_config.proxy_method, "openvpn");
    assert_eq!(data.cloak_config.remote_host, "203.0.113.1");
    assert_eq!(data.cloak_config.remote_port, "443");
    assert!(data.openvpn_config.contains("client"));
    assert!(data.openvpn_config.contains("dev tun"));
    assert!(data.openvpn_config.contains("remote 127.0.0.1 1194"));
}

#[test]
fn test_import_manual() {
    let ovpn_path = format!("{}/client.ovpn", fixtures_dir());
    let cloak_path = format!("{}/cloak.json", fixtures_dir());

    let data = import::import(ImportSource::Manual {
        ovpn_path,
        cloak_path,
    })
    .unwrap();

    assert_eq!(data.profile.protocol_type, ProtocolType::OpenvpnCloak);
    assert!(data.openvpn_config.contains("client"));
    assert!(data.openvpn_config.contains("dev tun"));
}

#[test]
fn test_full_import_and_store() {
    let tmp = tempfile::tempdir().unwrap();
    let store = ProfileStore::with_base_dir(tmp.path().to_path_buf()).unwrap();

    let backup_path = format!("{}/amnezia-openvpn-cloak.backup", fixtures_dir());
    let mut data = import::import(ImportSource::AmneziaBackup {
        path: backup_path,
    })
    .unwrap();

    data.profile.name = "test-amnezia".to_string();

    store.save(&data).unwrap();

    let loaded = store.load("test-amnezia").unwrap();
    assert_eq!(loaded.name, "test-amnezia");
    assert_eq!(loaded.protocol_type, ProtocolType::OpenvpnCloak);

    let ovpn = store.load_openvpn_config("test-amnezia").unwrap();
    assert!(ovpn.contains("client"));

    let cloak = store.load_cloak_config("test-amnezia").unwrap();
    assert!(cloak.contains("PublicKey"));
}

#[test]
fn test_cloak_config_validate() {
    let valid = CloakConfig {
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
    };
    assert!(valid.validate().is_ok());

    let missing_fields = CloakConfig {
        public_key: String::new(),
        uid: String::new(),
        remote_host: String::new(),
        remote_port: String::new(),
        proxy_method: String::new(),
        ..valid.clone()
    };
    assert!(missing_fields.validate().is_err());
}

#[test]
fn test_profile_name_validation() {
    assert!(Profile::sanitize_name("home").is_ok());
    assert!(Profile::sanitize_name("my-server-1").is_ok());
    assert!(Profile::sanitize_name("vpn_work").is_ok());
    assert!(Profile::sanitize_name("").is_err());
    assert!(Profile::sanitize_name("has space").is_err());
    assert!(Profile::sanitize_name("has/slash").is_err());
}