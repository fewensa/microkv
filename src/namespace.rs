use secstr::SecVec;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sodiumoxide::crypto::secretbox::{self, Key};

use crate::errors::{ErrorType, KVError, Result};
use crate::history::KV;
use crate::kv::Value;
use crate::MicroKV;

// Debug,
#[derive(Clone)]
pub struct NamespaceMicroKV {
    /// namespace
    namespace: String,
    /// stores the actual key-value store encapsulated with a RwLock
    microkv: MicroKV,
}

impl NamespaceMicroKV {
    pub fn new(namespace: impl AsRef<str>, microkv: MicroKV) -> Self {
        Self {
            namespace: namespace.as_ref().to_string(),
            microkv,
        }
    }

    fn key(&self, key: impl AsRef<str>) -> String {
        key.as_ref().to_string()
    }
}

impl NamespaceMicroKV {
    pub fn get_as<V>(&self, key: impl AsRef<str>) -> Result<Option<V>>
    where
        V: DeserializeOwned + 'static,
    {
        match self.get(key)? {
            Some(v) => Ok(Some(serde_json::from_value(v)?)),
            None => Ok(None),
        }
    }

    pub fn get_as_unwrap<V>(&self, key: impl AsRef<str>) -> Result<V>
    where
        V: DeserializeOwned + 'static,
    {
        if let Some(v) = self.get_as(key)? {
            return Ok(v);
        }
        Err(KVError {
            error: ErrorType::KVError,
            msg: Some("key not found in storage".to_string()),
        })
    }

    /// unsafe get, may this api can change name to get_unwrap
    pub fn get_unwrap(&self, key: impl AsRef<str>) -> Result<Value> {
        if let Some(v) = self.get(key)? {
            return Ok(v);
        }
        Err(KVError {
            error: ErrorType::KVError,
            msg: Some("key not found in storage".to_string()),
        })
    }

    /// Decrypts and retrieves a value. Can return errors if lock is poisoned,
    /// ciphertext decryption doesn't work, and if parsing bytes fail.
    pub fn get(&self, key: impl AsRef<str>) -> Result<Option<Value>> {
        let data_key = self.key(key);
        let value = self.microkv.lock_read(&self.namespace, |kv| {
            // initialize a copy of state
            let data = kv.clone();

            // retrieve value from IndexMap if stored, decrypt and return
            match data.get(&data_key) {
                Some(val) => {
                    // get value to deserialize. If password is set, retrieve the value, and decrypt it
                    // using AEAD. Otherwise just get the value and return
                    let deser_val = match &self.microkv.pwd() {
                        Some(pwd) => {
                            // initialize key from pwd slice
                            let key = match Key::from_slice(pwd.unsecure()) {
                                Some(k) => k,
                                None => {
                                    return Err(KVError {
                                        error: ErrorType::CryptoError,
                                        msg: Some(
                                            "cannot derive key from password hash".to_string(),
                                        ),
                                    });
                                }
                            };

                            // borrow secured value by reference, and decrypt before deserializing
                            match secretbox::open(val.unsecure(), self.microkv.nonce(), &key) {
                                Ok(r) => r,
                                Err(_) => {
                                    return Err(KVError {
                                        error: ErrorType::CryptoError,
                                        msg: Some(
                                            "cannot validate value being decrypted".to_string(),
                                        ),
                                    });
                                }
                            }
                        }

                        // if no password, return value as-is
                        None => val.unsecure().to_vec(),
                    };

                    // finally deserialize into deserializable object to return as
                    let value: String = bincode::deserialize(&deser_val).map_err(|e| KVError {
                        error: ErrorType::KVError,
                        msg: Some(format!(
                            "cannot deserialize into specified object type: {:?}",
                            e
                        )),
                    })?;
                    let value = serde_json::from_str(&value)?;
                    Ok(Some(value))
                }

                None => Ok(None),
            }
        })??;
        Ok(value)
    }

    /// Encrypts and adds a new key-value pair to storage.
    pub fn put<V>(&self, key: impl AsRef<str>, value: &V) -> Result<()>
    where
        V: Serialize,
    {
        let value = serde_json::to_value(value)?.to_string();
        let data_key = self.key(key);
        self.microkv.lock_write(&self.namespace, |data: &mut KV| {
            // to retain best-case constant runtime, we remove the key-value if found
            if data.contains_key(&data_key) {
                let _ = data.remove(&data_key).unwrap();
            }

            // serialize the object for committing to db
            let ser_val: Vec<u8> = bincode::serialize(&value).unwrap();

            // encrypt and secure value if password is available
            let value: SecVec<u8> = match self.microkv.pwd() {
                // encrypt using AEAD and secure memory
                Some(pwd) => {
                    let key: Key = Key::from_slice(pwd.unsecure()).unwrap();
                    SecVec::new(secretbox::seal(&ser_val, self.microkv.nonce(), &key))
                }

                // otherwise initialize secure serialized object to insert to BTreeMap
                None => SecVec::new(ser_val),
            };
            data.insert(data_key.clone(), value);
        })?;
        if !self.microkv.is_auto_commit() {
            return Ok(());
        }
        self.microkv.commit()
    }

    /// Delete removes an entry in the key value store.
    pub fn delete(&self, key: impl AsRef<str>) -> Result<()> {
        let data_key = self.key(key);
        self.microkv.lock_write(&self.namespace, |data| {
            // delete entry from BTreeMap by key
            let _ = data.remove(&data_key);
        })?;

        if !self.microkv.is_auto_commit() {
            return Ok(());
        }
        self.microkv.commit()
    }

    /// Helper routine that acquires a reader lock and checks if a key exists.
    pub fn exists(&self, key: impl AsRef<str>) -> Result<bool> {
        let data_key = self.key(key);
        self.microkv
            .lock_read(&self.namespace, |data| data.contains_key(&data_key))
    }

    /// Safely consumes an iterator over the keys in the `IndexMap` and returns a
    /// `Vec<String>` for further use.
    ///
    /// Note that key iteration, not value iteration, is only supported in order to preserve
    /// security guarentees.
    pub fn keys(&self) -> Result<Vec<String>> {
        let keys = self.microkv.lock_read(&self.namespace, |kv| {
            // initialize a copy to data
            let data = kv.clone();
            data.keys().map(|x| x.to_string()).collect::<Vec<String>>()
        })?;
        Ok(keys)
    }

    /// Safely consumes an iterator over a copy of in-place sorted keys in the
    /// `IndexMap` and returns a `Vec<String>` for further use.
    ///
    /// Note that key iteration, not value iteration, is only supported in order to preserve
    /// security guarentees.
    pub fn sorted_keys(&self) -> Result<Vec<String>> {
        let keys = self.microkv.lock_read(&self.namespace, |kv| {
            // initialize a copy to data, and sort keys in-place
            let mut data = kv.clone();
            data.sort_keys();
            data.keys().map(|x| x.to_string()).collect::<Vec<String>>()
        })?;
        Ok(keys)
    }

    /// Empties out the entire underlying `IndexMap` in O(n) time, but does
    /// not delete the persistent storage file from disk. The `IndexMap` remains,
    /// and its capacity is kept the same.
    pub fn clear(&self) -> Result<()> {
        self.microkv.lock_write(&self.namespace, |data| {
            // first, iterate over the IndexMap and coerce drop on the secure value wrappers
            for (_, value) in data.iter_mut() {
                value.zero_out();
            }

            // next, clear all entries from the IndexMap
            data.clear();
        })?;

        // auto commit
        if !self.microkv.is_auto_commit() {
            return Ok(());
        }
        self.microkv.commit()
    }
}
