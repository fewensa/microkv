use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use crate::errors::{ErrorType, KVError, Result};
use crate::{history, MicroKV};

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
            .or_else(|_e| self.try_less_than_030(&kv_raw));
        match ret {
            Ok(v) => Ok(v),
            Err(e) => match e.error {
                ErrorType::MigrateError(from, to) => Err(KVError {
                    error: ErrorType::MigrateError(from.clone(), to.clone()),
                    msg: Some(format!(
                        "Not support migrate {:?} from {} to {}",
                        self.path, from, to
                    )),
                }),
                _ => Err(KVError {
                    error: ErrorType::MigrateError(
                        "UNKNOWN".to_string(),
                        CURRENT_VERSION.to_string(),
                    ),
                    msg: Some(format!(
                        "Not support migrate {:?} from UNKNOWN to {}",
                        self.path, CURRENT_VERSION
                    )),
                }),
            },
        }
    }

    fn try_current(&self, binary: &[u8]) -> Result<history::MicroKV030> {
        bincode::deserialize(binary).map_err(|_e| KVError {
            error: ErrorType::MigrateError("0.3.0".to_string(), CURRENT_VERSION.to_string()),
            msg: Some("Failed to deserialize to 0.3.0".to_string()),
        })
    }

    fn try_less_than_030(&self, _binary: &[u8]) -> Result<MicroKV> {
        Err(KVError {
            error: ErrorType::MigrateError("<0.3.0".to_string(), CURRENT_VERSION.to_string()),
            msg: Some(format!(
                "Not support migrate less than 0.3.0 to {}",
                CURRENT_VERSION
            )),
        })
    }
}
