use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use indexmap::IndexMap;
use secstr::{SecStr, SecVec};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::secretbox::Nonce;

use crate::history::builder::MicroKV030Builder;

/// An alias to a base data structure that supports storing
/// associated types. An `IndexMap` is a strong choice due to
/// strong asymptotic performance with sorted key iteration.
pub(crate) type KV = IndexMap<String, SecVec<u8>>;
pub(crate) type Storage = Arc<RwLock<KV>>;

/// The MicroKV class version less than 0.3.0
#[derive(Clone, Serialize, Deserialize)]
pub struct MicroKVLessThan030 {
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

impl MicroKV030 {
    pub fn builder() -> MicroKV030Builder {
        MicroKV030Builder::new()
    }
}

pub mod builder {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, RwLock};

    use secstr::{SecStr, SecVec};
    use sodiumoxide::crypto::secretbox;
    use sodiumoxide::crypto::secretbox::Nonce;

    use crate::history::{MicroKV030, MicroKVLessThan030, Storage, KV};

    pub struct MicroKVLessThan030Builder {
        inner: MicroKVLessThan030,
    }

    impl MicroKVLessThan030Builder {
        pub(crate) fn new() -> Self {
            Self {
                inner: MicroKVLessThan030 {
                    path: Default::default(),
                    storage: Arc::new(RwLock::new(Default::default())),
                    nonce: secretbox::gen_nonce(),
                    pwd: None,
                    is_auto_commit: false,
                },
            }
        }

        pub fn build(&self) -> MicroKVLessThan030 {
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

    pub struct MicroKV030Builder {
        inner: MicroKV030,
    }

    impl MicroKV030Builder {
        pub(crate) fn new() -> Self {
            Self {
                inner: MicroKV030 {
                    version: "0.3.0".to_string(),
                    path: Default::default(),
                    storage: Arc::new(RwLock::new(Default::default())),
                    nonce: secretbox::gen_nonce(),
                    pwd: None,
                    is_auto_commit: false,
                },
            }
        }

        pub fn build(&self) -> MicroKV030 {
            self.inner.clone()
        }

        pub fn path(&mut self, path: PathBuf) -> &mut Self {
            self.inner.path = path;
            self
        }

        pub fn storage(&mut self, storage: HashMap<String, Storage>) -> &mut Self {
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
}
