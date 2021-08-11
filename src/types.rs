use std::sync::{Arc, RwLock};

use indexmap::IndexMap;
use secstr::SecVec;

/// An alias to a base data structure that supports storing
/// associated types. An `IndexMap` is a strong choice due to
/// strong asymptotic performance with sorted key iteration.
pub type KV = IndexMap<String, SecVec<u8>>;
pub type Storage = Arc<RwLock<KV>>;
