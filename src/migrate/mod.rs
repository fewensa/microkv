use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use crate::errors::{ErrorType, KVError, Result};
use crate::MicroKV;

pub mod history;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Migrate {
    path: PathBuf,
}

impl Migrate {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Migrate {
    pub fn migrate(&self) -> Result<MicroKV> {
        // read kv raw serialized structure to kv_raw
        let mut kv_raw: Vec<u8> = Vec::new();
        File::open(&self.path)?.read_to_end(&mut kv_raw)?;

        let ret = self
            .try_current(&kv_raw)
            .or_else(|_e| self.try_less_than_027(&kv_raw));
        match ret {
            Ok(v) => Ok(v),
            Err(_e) => Err(KVError {
                error: ErrorType::MigrateError,
                msg: Some(format!(
                    "Not support migrate {:?} from UNKNOWN to {}",
                    self.path,
                    env!("CARGO_PKG_VERSION")
                )),
            }),
        }
    }

    fn try_current(&self, binary: &[u8]) -> Result<history::MicroKV027> {
        bincode::deserialize(binary).map_err(|_e| KVError {
            error: ErrorType::MigrateError,
            msg: Some("Failed to deserialize to 0.2.7".to_string()),
        })
    }

    fn try_less_than_027(&self, binary: &[u8]) -> Result<MicroKV> {
        // deserialize with bincode and return
        let kv_less_than_027: history::MicroKVLessThan027 =
            bincode::deserialize(binary).map_err(|_e| KVError {
                error: ErrorType::MigrateError,
                msg: Some("Failed to deserialize to < 0.2.7".to_string()),
            })?;
        from_less_than_027::FromLessThan027::new(kv_less_than_027, CURRENT_VERSION).migrate()
    }
}

mod from_less_than_027 {
    use std::sync::{Arc, RwLock};

    use crate::errors::{ErrorType, KVError, Result};

    use super::history;
    use std::collections::HashMap;

    pub struct FromLessThan027 {
        kv: history::MicroKVLessThan027,
        target: String,
    }

    impl FromLessThan027 {
        pub fn new(kv: history::MicroKVLessThan027, target: impl AsRef<str>) -> Self {
            Self {
                kv,
                target: target.as_ref().to_string(),
            }
        }
    }

    impl FromLessThan027 {
        pub fn migrate(&self) -> Result<history::MicroKV027> {
            match &self.target[..] {
                "0.2.7" => self.migrate_to_027(),
                _ => Err(KVError {
                    error: ErrorType::MigrateError,
                    msg: Some(format!(
                        "Not support migrate from [less then 0.2.7] to {}",
                        self.target
                    )),
                }),
            }
        }
        fn migrate_to_027(&self) -> Result<history::MicroKV027> {
            let mut storage_map = HashMap::new();
            storage_map.insert("".to_string(), self.kv.storage.clone());
            let storage = Arc::new(RwLock::new(storage_map));
            let microkv = history::MicroKV027 {
                version: "0.2.7".to_string(),
                path: self.kv.path.clone(),
                storage,
                nonce: self.kv.nonce,
                pwd: self.kv.pwd.clone(),
                is_auto_commit: self.kv.is_auto_commit,
            };
            microkv.commit()?;
            Ok(microkv)
        }
    }
}
