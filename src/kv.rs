//! Defines the foundational structure and API for the key-value store implementation.
//! The `kv` module should be used to spin up localized instances of the key-value store.
//!
//! ## Features
//!
//! * Database interaction operations, with sorted-key iteration possible
//! * Serialization to persistent storage
//! * Symmetric authenticated cryptography
//! * Mutual exclusion with RWlocks and mutexes
//! * Secure memory wiping
//!
//! ## Example
//!
//! ```rust
//! use microkv::MicroKV;
//!
//! let kv: MicroKV = MicroKV::new("example").with_pwd_clear("p@ssw0rd".to_string());
//!
//! // put
//! let value = 123;
//! kv.put("keyname", &value);
//!
//! // get
//! let res: i32 = kv.get_as_unwrap("keyname").expect("cannot retrieve value");
//! println!("{}", res);
//!
//! // delete
//! kv.delete("keyname").expect("cannot delete key");
//! ```
//!
//! width namespace
//!
//! ```rust
//! use microkv::MicroKV;
//!
//! let kv: MicroKV = MicroKV::new("example").with_pwd_clear("p@ssw0rd".to_string());
//! let namespace_custom = kv.namespace("custom");
//!
//! // put
//! let value = 123;
//! namespace_custom.put("keyname", &value);
//!
//! // get
//! let res: i32 = namespace_custom.get_as_unwrap("keyname").expect("cannot retrieve value");
//! println!("{}", res);
//!
//! // delete
//! namespace_custom.delete("keyname").expect("cannot delete key");
//! ```
#![allow(clippy::result_map_unit_fn)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use secstr::{SecStr, SecVec};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sodiumoxide::crypto::hash::sha256;
use sodiumoxide::crypto::secretbox::{self, Nonce};

use crate::errors::{ErrorType, KVError, Result};
use crate::helpers;
use crate::migrate::Migrate;
use crate::namespace::NamespaceMicroKV;

pub type Value = serde_json::Value;
pub type MicroKV = crate::history::MicroKV030;

impl MicroKV {
    /// New MicroKV store with store to base path
    pub fn new_with_base_path<S: AsRef<str>>(dbname: S, base_path: PathBuf) -> Self {
        let storage = Arc::new(RwLock::new(HashMap::new()));

        // no password, until set by `with_pwd_*` methods
        let pwd: Option<SecStr> = None;

        // initialize a new public nonce for symmetric AEAD
        let nonce: Nonce = secretbox::gen_nonce();

        // get abspath to dbname to write to.
        let path = helpers::get_db_path_with_base_path(dbname, base_path);

        Self::create(path, pwd, nonce, false, storage)
    }

    /// Initializes a new empty and unencrypted MicroKV store with
    /// an identifying database name. This is the bare minimum that can operate as a
    /// key-value store, and can be configured using other builder methods.
    pub fn new<S: AsRef<str>>(dbname: S) -> Self {
        let mut path = helpers::get_home_dir();
        path.push(helpers::DEFAULT_WORKSPACE_PATH);
        Self::new_with_base_path(dbname, path)
    }

    /// Open with base path
    pub fn open_with_base_path<S: AsRef<str>>(dbname: S, base_path: PathBuf) -> Result<Self> {
        // initialize abspath to persistent db
        let path = helpers::get_db_path_with_base_path(dbname.as_ref(), base_path.clone());

        if path.is_file() {
            let migrate = Migrate::new(path.clone());
            let mut kv = migrate.migrate()?;
            kv.path = path;
            kv.commit()?;
            Ok(kv)
        } else {
            Ok(Self::new_with_base_path(dbname, base_path))
        }
    }

    /// Opens a previously instantiated and encrypted MicroKV, given a db name.
    /// The public nonce generated from a previous session is also retrieved in order to
    /// do authenticated encryption later on.
    pub fn open<S: AsRef<str>>(dbname: S) -> Result<Self> {
        let mut path = helpers::get_home_dir();
        path.push(helpers::DEFAULT_WORKSPACE_PATH);
        Self::open_with_base_path(dbname, path)
    }

    /*
    /// `override_path()` changes the default path for persisting the store, rather than
    /// writing/reading from the default workspace directory.
    pub fn override_path(mut self, path: PathBuf) -> io::Result<Self> {
        self.path = fs::canonicalize(Path::new(&path))?;
        Ok(self)
    }
    */

    /// Builds up the MicroKV with a cleartext password, which is hashed using
    /// the defaultly supported SHA-256 by `sodiumoxide`, in order to instantiate a 32-byte hash.
    ///
    /// Use if the password to encrypt is not naturally pseudorandom and secured in-memory,
    /// and is instead read elsewhere, like a file or stdin (developer should guarentee security when
    /// implementing such methods, as MicroKV only guarentees hashing and secure storage).
    pub fn with_pwd_clear<S: AsRef<str>>(mut self, unsafe_pwd: S) -> Self {
        let pwd: SecStr = SecVec::new(sha256::hash(unsafe_pwd.as_ref().as_bytes()).0.to_vec());
        self.pwd = Some(pwd);
        self
    }

    /// Builds up the MicroKV with a hashed buffer, which is then locked securely `for later use.
    ///
    /// Use if the password to encrypt is generated as a pseudorandom value, or previously hashed by
    /// another preferred one-way function within or outside the application.
    pub fn with_pwd_hash(mut self, _pwd: [u8; 32]) -> Self {
        let pwd: SecStr = SecVec::new(_pwd.to_vec());
        self.pwd = Some(pwd);
        self
    }

    /// Set is auto commit
    pub fn set_auto_commit(mut self, enable: bool) -> Self {
        self.is_auto_commit = enable;
        self
    }

    ///////////////////////////////////////
    // extended
    ///////////////////////////////////////

    pub fn namespaces(&self) -> Result<Vec<String>> {
        let storage = self.storage.read().map_err(|_| KVError {
            error: ErrorType::PoisonError,
            msg: None,
        })?;
        let keys = storage.keys().cloned().collect::<Vec<String>>();
        Ok(keys)
    }

    pub fn namespace(&self, namespace: impl AsRef<str>) -> NamespaceMicroKV {
        NamespaceMicroKV::new(namespace, self.clone())
    }

    pub fn namespace_default(&self) -> NamespaceMicroKV {
        self.namespace("")
    }

    ///////////////////////////////////////
    // Primitive key-value store operations
    ///////////////////////////////////////

    pub fn get_as<V>(&self, key: impl AsRef<str>) -> Result<Option<V>>
    where
        V: DeserializeOwned + 'static,
    {
        self.namespace_default().get_as(key)
    }

    pub fn get_as_unwrap<V>(&self, key: impl AsRef<str>) -> Result<V>
    where
        V: DeserializeOwned + 'static,
    {
        self.namespace_default().get_as_unwrap(key)
    }

    /// unsafe get, may this api can change name to get_unwrap
    pub fn get_unwrap(&self, key: impl AsRef<str>) -> Result<Value> {
        self.namespace_default().get_unwrap(key)
    }

    /// Decrypts and retrieves a value. Can return errors if lock is poisoned,
    /// ciphertext decryption doesn't work, and if parsing bytes fail.
    pub fn get(&self, key: impl AsRef<str>) -> Result<Option<Value>> {
        self.namespace_default().get(key)
    }

    /// Encrypts and adds a new key-value pair to storage.
    pub fn put<V>(&self, key: impl AsRef<str>, value: &V) -> Result<()>
    where
        V: Serialize,
    {
        self.namespace_default().put(key, value)
    }

    /// Delete removes an entry in the key value store.
    pub fn delete(&self, key: impl AsRef<str>) -> Result<()> {
        self.namespace_default().delete(key)
    }

    //////////////////////////////////////////
    // Other key-value store helper operations
    //////////////////////////////////////////

    /// Helper routine that acquires a reader lock and checks if a key exists.
    pub fn exists(&self, key: impl AsRef<str>) -> Result<bool> {
        self.namespace_default().exists(key)
    }

    /// Safely consumes an iterator over the keys in the `IndexMap` and returns a
    /// `Vec<String>` for further use.
    ///
    /// Note that key iteration, not value iteration, is only supported in order to preserve
    /// security guarentees.
    pub fn keys(&self) -> Result<Vec<String>> {
        self.namespace_default().keys()
    }

    /// Safely consumes an iterator over a copy of in-place sorted keys in the
    /// `IndexMap` and returns a `Vec<String>` for further use.
    ///
    /// Note that key iteration, not value iteration, is only supported in order to preserve
    /// security guarentees.
    pub fn sorted_keys(&self) -> Result<Vec<String>> {
        self.namespace_default().sorted_keys()
    }

    /// Empties out the entire underlying `IndexMap` in O(n) time, but does
    /// not delete the persistent storage file from disk. The `IndexMap` remains,
    /// and its capacity is kept the same.
    pub fn clear(&self) -> Result<()> {
        self.namespace_default().clear()
    }
}

// coerce a secure zero wipe
impl Drop for MicroKV {
    fn drop(&mut self) {
        if let Some(ref mut pwd) = self.pwd {
            pwd.zero_out()
        }
    }
}
