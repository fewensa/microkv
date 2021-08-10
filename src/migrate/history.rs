use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use indexmap::IndexMap;
use secstr::{SecStr, SecVec};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::secretbox::Nonce;

pub(crate) type KV = IndexMap<String, SecVec<u8>>;
pub(crate) type Storage = Arc<RwLock<KV>>;

/// The MicroKV class version less than 0.2.7
#[derive(Clone, Serialize, Deserialize)]
pub struct MicroKVLessThan027 {
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

/// The MicroKV class version 0.2.7
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
