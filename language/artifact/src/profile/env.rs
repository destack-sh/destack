use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// Comptime environment snapshot used for profile identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvSnapshot {
    /// Full environment snapshot with keys and hashed values.
    All { keys: Vec<String>, hash: u64 },
    /// Whitelisted environment snapshot with keys and hashed values.
    Whitelist { keys: Vec<String>, hash: u64 },
}

impl EnvSnapshot {
    /// Snapshot all environment keys and values.
    pub fn from_env_all() -> Self {
        let mut entries: Vec<(String, String)> = std::env::vars().collect();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let keys = entries.iter().map(|(key, _)| key.clone()).collect();
        let hash = hash_env_entries(&entries);
        Self::All { keys, hash }
    }

    /// Snapshot only whitelisted environment keys and values.
    pub fn from_env_whitelist(keys: &[String]) -> Self {
        let keys = normalize_profile_keys(keys.to_vec());
        let mut entries = Vec::with_capacity(keys.len());
        for key in &keys {
            let value = std::env::var(key).unwrap_or_default();
            entries.push((key.clone(), value));
        }
        let hash = hash_env_entries(&entries);
        Self::Whitelist { keys, hash }
    }

    /// Return the environment keys included in this snapshot.
    pub fn keys(&self) -> &[String] {
        match self {
            Self::All { keys, .. } | Self::Whitelist { keys, .. } => keys,
        }
    }
}

/// Normalize profile keys by sorting and deduplicating them.
pub(crate) fn normalize_profile_keys(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys.dedup();
    keys
}

/// Hash environment entries by hashing each key and value pair.
fn hash_env_entries(entries: &[(String, String)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (key, value) in entries {
        key.hash(&mut hasher);
        value.hash(&mut hasher);
    }
    hasher.finish()
}
