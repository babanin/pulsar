pub mod amnezia;
pub mod manual;

use crate::error::Result;
use crate::profile::model::ProfileData;

pub enum ImportSource {
    AmneziaBackup { path: String },
    #[allow(dead_code)]
    AmneziaFileContents { contents: String },
    Manual {
        ovpn_path: String,
        cloak_path: String,
    },
}

pub fn import(source: ImportSource) -> Result<ProfileData> {
    match source {
        ImportSource::AmneziaBackup { path } => {
            let contents = std::fs::read_to_string(&path)?;
            amnezia::import_amnezia_backup(&contents)
        }
        ImportSource::AmneziaFileContents { contents } => {
            amnezia::import_amnezia_backup(&contents)
        }
        ImportSource::Manual { ovpn_path, cloak_path } => {
            manual::import_manual(&ovpn_path, &cloak_path)
        }
    }
}