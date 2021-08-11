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
    pub(crate) version: String,
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

// impl MicroKV030 {
//     pub fn builder() -> MicroKV030Builder {
//         MicroKV030Builder::new()
//     }
//
//     pub fn version(&self) -> &String {
//         &self.version
//     }
//
//     pub fn path(&self) -> &PathBuf {
//         &self.path
//     }
//
//     pub fn storage(&self) -> &Arc<RwLock<HashMap<String, Storage>>> {
//         &self.storage
//     }
//
//     pub fn is_auto_commit(&self) -> bool {
//         self.is_auto_commit
//     }
//
//     pub fn pwd(&self) -> &Option<SecStr> {
//         &self.pwd
//     }
//
//     pub fn nonce(&self) -> &Nonce {
//         &self.nonce
//     }
// }

impl MicroKV030 {
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

// #[derive(Clone)]
// pub struct MicroKV030Builder {
//     inner: MicroKV030,
// }
//
// impl MicroKV030Builder {
//     pub(crate) fn new() -> Self {
//         Self {
//             inner: MicroKV030 {
//                 version: "0.3.0".to_string(),
//                 path: Default::default(),
//                 storage: Arc::new(RwLock::new(Default::default())),
//                 nonce: secretbox::gen_nonce(),
//                 pwd: None,
//                 is_auto_commit: false,
//             },
//         }
//     }
//
//     pub fn build(&self) -> MicroKV030 {
//         self.inner.clone()
//     }
//
//     pub fn path(&mut self, path: PathBuf) -> &mut Self {
//         self.inner.path = path;
//         self
//     }
//
//     pub fn storage(&mut self, storage: HashMap<String, Storage>) -> &mut Self {
//         self.inner.storage = Arc::new(RwLock::new(storage));
//         self
//     }
//
//     pub fn nonce(&mut self, nonce: Nonce) -> &mut Self {
//         self.inner.nonce = nonce;
//         self
//     }
//
//     pub fn pwd(&mut self, pwd: Option<SecStr>) -> &mut Self {
//         self.inner.pwd = pwd;
//         self
//     }
//
//     pub fn is_auto_commit(&mut self, is_auto_commit: bool) -> &mut Self {
//         self.inner.is_auto_commit = is_auto_commit;
//         self
//     }
// }
