use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use secstr::{SecStr, SecVec};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sodiumoxide::crypto::secretbox::Nonce;
use sodiumoxide::crypto::secretbox::{self, Key};

use crate::errors::{ErrorType, KVError, Result};

/// Defines the directory path where a key-value store
/// (or multiple) can be interacted with.
pub(crate) const DEFAULT_WORKSPACE_PATH: &str = ".microkv/";

/// Helper that retrieves the home directory by resolving $HOME
#[inline]
pub fn get_home_dir() -> PathBuf {
    dirs::home_dir().unwrap()
}

/// Helper that forms an absolute path from a given database name and the default workspace path.
#[inline]
pub fn get_db_path<S: AsRef<str>>(name: S) -> PathBuf {
    let mut path = get_home_dir();
    path.push(DEFAULT_WORKSPACE_PATH);
    get_db_path_with_base_path(name, path)
}

/// with base path
#[inline]
pub fn get_db_path_with_base_path<S: AsRef<str>>(name: S, mut base_path: PathBuf) -> PathBuf {
    base_path.push(name.as_ref());
    base_path.set_extension("kv");
    base_path
}

/// read file and deserialize use bincode
#[inline]
pub fn read_file_and_deserialize_bincode<V>(path: &PathBuf) -> Result<V>
where
    V: DeserializeOwned + 'static,
{
    // read kv raw serialized structure to kv_raw
    let mut kv_raw: Vec<u8> = Vec::new();
    File::open(path)?.read_to_end(&mut kv_raw)?;
    bincode::deserialize(&kv_raw).map_err(|_e| KVError {
        error: ErrorType::FileError,
        msg: Some(format!(
            "Failed read file {:?} an deserialize use bincode",
            path
        )),
    })
}

/// gen nonce
pub fn gen_nonce() -> Nonce {
    secretbox::gen_nonce()
}

/// encode value
pub fn encode_value<V>(value: &V, pwd: &Option<SecStr>, nonce: &Nonce) -> Result<SecVec<u8>>
where
    V: Serialize,
{
    // all data serialize to serde_json::Value
    let value = serde_json::to_value(value)?.to_string();
    // serialize the object for committing to db
    let ser_val: Vec<u8> = bincode::serialize(&value).unwrap();
    // encrypt and secure value if password is available
    let value: SecVec<u8> = match pwd {
        // encrypt using AEAD and secure memory
        Some(pwd) => {
            let key: Key = Key::from_slice(pwd.unsecure()).unwrap();
            SecVec::new(secretbox::seal(&ser_val, nonce, &key))
        }

        // otherwise initialize secure serialized object to insert to BTreeMap
        None => SecVec::new(ser_val),
    };
    Ok(value)
}

/// decode value
pub fn decode_value(
    value: &SecVec<u8>,
    pwd: &Option<SecStr>,
    nonce: &Nonce,
) -> Result<serde_json::Value> {
    // get value to deserialize. If password is set, retrieve the value, and decrypt it
    // using AEAD. Otherwise just get the value and return
    let deser_val = match pwd {
        Some(pwd) => {
            // initialize key from pwd slice
            let key = match Key::from_slice(pwd.unsecure()) {
                Some(k) => k,
                None => {
                    return Err(KVError {
                        error: ErrorType::CryptoError,
                        msg: Some("cannot derive key from password hash".to_string()),
                    });
                }
            };

            // borrow secured value by reference, and decrypt before deserializing
            match secretbox::open(value.unsecure(), nonce, &key) {
                Ok(r) => r,
                Err(_) => {
                    return Err(KVError {
                        error: ErrorType::CryptoError,
                        msg: Some("cannot validate value being decrypted".to_string()),
                    });
                }
            }
        }

        // if no password, return value as-is
        None => value.unsecure().to_vec(),
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
    Ok(value)
}

/// Writes the IndexMap to persistent storage after encrypting with secure crypto construction.
pub(crate) fn persist_serialize<S>(path: &PathBuf, object: &S) -> Result<()>
where
    S: Serialize,
{
    // initialize workspace directory if not exists
    match path.parent() {
        Some(path) => {
            if !path.is_dir() {
                std::fs::create_dir_all(path)?;
            }
        }
        None => {
            return Err(KVError {
                error: ErrorType::FileError,
                msg: Some("The store file parent path isn't sound".to_string()),
            });
        }
    }

    // check if path to db exists, if not create it
    let path = Path::new(path);
    let mut file: File = OpenOptions::new().write(true).create(true).open(path)?;

    // acquire a file lock that unlocks at the end of scope
    // let _file_lock = Arc::new(Mutex::new(0));
    let ser = bincode::serialize(object).unwrap();
    file.write_all(&ser)?;
    Ok(())
}
