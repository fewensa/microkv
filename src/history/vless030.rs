use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use secstr::SecStr;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::secretbox;
use sodiumoxide::crypto::secretbox::Nonce;

use crate::errors::{ErrorType, KVError, Result};
use crate::types::KV;

/// The MicroKV class version less than 0.3.0
#[derive(Clone, Serialize, Deserialize)]
pub struct MicroKVLess030 {
    pub(crate) path: PathBuf,

    /// stores the actual key-value store encapsulated with a RwLock
    pub(crate) storage: Arc<RwLock<KV>>,

    /// pseudorandom nonce that can be publicly known
    pub(crate) nonce: Nonce,

    /// memory-guarded hashed password
    #[serde(skip_serializing, skip_deserializing)]
    pub(crate) pwd: Option<SecStr>,

    /// is auto commit
    pub(crate) is_auto_commit: bool,
}

impl MicroKVLess030 {
    pub fn builder() -> MicroKVLessThan030Builder {
        MicroKVLessThan030Builder::new()
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn storage(&self) -> &Arc<RwLock<KV>> {
        &self.storage
    }

    pub fn is_auto_commit(&self) -> bool {
        self.is_auto_commit
    }

    pub fn pwd(&self) -> &Option<SecStr> {
        &self.pwd
    }

    pub fn nonce(&self) -> &Nonce {
        &self.nonce
    }
}

impl MicroKVLess030 {
    /// Arbitrary read-lock that encapsulates a read-only closure. Multiple concurrent readers
    /// can hold a lock and parse out data.
    pub fn lock_read<C, R>(&self, callback: C) -> Result<R>
    where
        C: Fn(&KV) -> R,
    {
        let data = self.storage.read().map_err(|_| KVError {
            error: ErrorType::PoisonError,
            msg: None,
        })?;
        Ok(callback(&data))
    }

    /// Arbitrary write-lock that encapsulates a write-only closure Single writer can hold a
    /// lock and mutate data, blocking any other readers/writers before the lock is released.
    pub fn lock_write<C, R>(&self, mut callback: C) -> Result<R>
    where
        C: FnMut(&KV) -> R,
    {
        let mut data = self.storage.write().map_err(|_| KVError {
            error: ErrorType::PoisonError,
            msg: None,
        })?;
        Ok(callback(&mut data))
    }
}

#[derive(Clone)]
pub struct MicroKVLessThan030Builder {
    inner: MicroKVLess030,
}

impl MicroKVLessThan030Builder {
    pub(crate) fn new() -> Self {
        Self {
            inner: MicroKVLess030 {
                path: Default::default(),
                storage: Arc::new(RwLock::new(Default::default())),
                nonce: secretbox::gen_nonce(),
                pwd: None,
                is_auto_commit: false,
            },
        }
    }

    pub fn build(&self) -> MicroKVLess030 {
        self.inner.clone()
    }

    pub fn path(&mut self, path: PathBuf) -> &mut Self {
        self.inner.path = path;
        self
    }

    pub fn storage(&mut self, storage: KV) -> &mut Self {
        self.inner.storage = Arc::new(RwLock::new(storage));
        self
    }

    pub fn nonce(&mut self, nonce: Nonce) -> &mut Self {
        self.inner.nonce = nonce;
        self
    }

    pub fn pwd(&mut self, pwd: Option<SecStr>) -> &mut Self {
        self.inner.pwd = pwd;
        self
    }

    pub fn is_auto_commit(&mut self, is_auto_commit: bool) -> &mut Self {
        self.inner.is_auto_commit = is_auto_commit;
        self
    }
}
