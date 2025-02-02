use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use secstr::{SecStr, SecVec};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::secretbox::Nonce;

use crate::errors::{ErrorType, KVError, Result};
use crate::helpers;
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
    pub fn encode_value<V>(&self, value: &V) -> Result<SecVec<u8>>
    where
        V: Serialize,
    {
        helpers::encode_value(value, &self.pwd, &self.nonce)
    }

    pub fn decode_value<V>(&self, value: &SecVec<u8>) -> Result<V>
    where
        V: DeserializeOwned + 'static,
    {
        helpers::decode_value(value, &self.pwd, &self.nonce)
    }

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
