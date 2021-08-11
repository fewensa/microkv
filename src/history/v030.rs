use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use secstr::{SecStr, SecVec};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::secretbox::Nonce;

use crate::errors::{ErrorType, KVError, Result};
use crate::helpers;
use crate::types::{Storage, KV};

/// The MicroKV class version 0.3.0
/// Defines the main interface structure to represent the most
/// recent state of the data store.
#[derive(Clone, Serialize, Deserialize)]
pub struct MicroKV030 {
    /// The version of persist data. this field will help migrate
    version: String,
    /// The version of persist data. this field will help migrate
    pub(crate) path: PathBuf,

    /// stores the actual key-value store encapsulated with a RwLock
    pub(crate) storage: Arc<RwLock<HashMap<String, Storage>>>,

    /// pseudorandom nonce that can be publicly known
    pub(crate) nonce: Nonce,

    /// memory-guarded hashed password
    #[serde(skip_serializing, skip_deserializing)]
    pub(crate) pwd: Option<SecStr>,

    /// is auto commit
    pub(crate) is_auto_commit: bool,
}

impl MicroKV030 {
    pub fn create(path: PathBuf, pwd: Option<SecStr>, nonce: Nonce, is_auto_commit: bool) -> Self {
        let storage = Arc::new(RwLock::new(HashMap::new()));
        Self {
            version: "0.3.0".to_string(),
            path,
            storage,
            nonce,
            pwd,
            is_auto_commit,
        }
    }
}

impl MicroKV030 {
    pub fn version(&self) -> &String {
        &self.version
    }

    pub fn encode_value<V>(&self, value: &V) -> Result<SecVec<u8>>
    where
        V: Serialize,
    {
        helpers::encode_value(value, &self.pwd, &self.nonce)
    }

    pub fn decode_value(&self, value: &SecVec<u8>) -> Result<serde_json::Value> {
        helpers::decode_value(value, &self.pwd, &self.nonce)
    }

    fn safe_storage(&self, namespace: impl AsRef<str>) -> Result<()> {
        let namespace = namespace.as_ref();
        let mut storage_map = self.storage.write().map_err(|_| KVError {
            error: ErrorType::PoisonError,
            msg: None,
        })?;
        if !storage_map.contains_key(namespace) {
            let storage = Arc::new(RwLock::new(KV::new()));
            storage_map.insert(namespace.to_string(), storage);
        }
        Ok(())
    }

    /// Arbitrary read-lock that encapsulates a read-only closure. Multiple concurrent readers
    /// can hold a lock and parse out data.
    pub fn lock_read<C, R>(&self, namespace: impl AsRef<str>, callback: C) -> Result<R>
    where
        C: Fn(&KV) -> R,
    {
        let namespace = namespace.as_ref();
        self.safe_storage(namespace)?;
        let storage_map = self.storage.read().map_err(|_| KVError {
            error: ErrorType::PoisonError,
            msg: None,
        })?;
        let storage = storage_map.get(namespace).unwrap();
        let data = storage.read().map_err(|_| KVError {
            error: ErrorType::PoisonError,
            msg: None,
        })?;
        Ok(callback(&data))
    }

    /// Arbitrary write-lock that encapsulates a write-only closure Single writer can hold a
    /// lock and mutate data, blocking any other readers/writers before the lock is released.
    pub fn lock_write<C, R>(&self, namespace: impl AsRef<str>, mut callback: C) -> Result<R>
    where
        C: FnMut(&mut KV) -> R,
    {
        let namespace = namespace.as_ref();
        self.safe_storage(namespace)?;
        let storage_map = self.storage.read().map_err(|_| KVError {
            error: ErrorType::PoisonError,
            msg: None,
        })?;
        let storage = storage_map.get(namespace).unwrap();
        let mut data = storage.write().map_err(|_| KVError {
            error: ErrorType::PoisonError,
            msg: None,
        })?;
        Ok(callback(&mut data))
    }

    ///////////////////
    // I/O Operations
    ///////////////////

    /// Writes the IndexMap to persistent storage after encrypting with secure crypto construction.
    pub fn commit(&self) -> Result<()> {
        helpers::persist_serialize(&self.path, self)
    }

    /// Clears the underlying data structure for the key-value store, and deletes the database file to remove all traces.
    pub fn destruct(&self) -> Result<()> {
        unimplemented!();
    }
}
