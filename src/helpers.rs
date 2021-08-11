use std::path::PathBuf;

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
