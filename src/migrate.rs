use std::path::PathBuf;

use crate::errors::{ErrorType, KVError, Result};
use crate::{helpers, history, MicroKV};

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
        let ret = self.try_current().or_else(|_e| self.try_less_than_030());
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

    fn try_current(&self) -> Result<history::MicroKV030> {
        helpers::read_file_and_deserialize_bincode(&self.path).map_err(|e| KVError {
            error: ErrorType::MigrateError("0.3.0".to_string(), CURRENT_VERSION.to_string()),
            msg: Some(format!("Failed to deserialize to 0.3.0 -> {:?}", e)),
        })
    }

    fn try_less_than_030(&self) -> Result<MicroKV> {
        Err(KVError {
            error: ErrorType::MigrateError("<0.3.0".to_string(), CURRENT_VERSION.to_string()),
            msg: Some(format!(
                "Not support migrate less than 0.3.0 to {}",
                CURRENT_VERSION
            )),
        })
    }
}
