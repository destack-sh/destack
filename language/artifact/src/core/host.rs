use destack_core::StableHasher;
use serde::{Deserialize, Serialize};

/// Stable identity for host environment variables read by one profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HostEnvironmentKey {
    /// All captured environment variables by key and hash.
    All { keys: Vec<String>, hash: u128 },
    /// Whitelisted environment variables by key and hash.
    Whitelist { keys: Vec<String>, hash: u128 },
}

impl HostEnvironmentKey {
    /// Build one key from all provided environment variables.
    pub fn all(entries: Vec<(String, String)>) -> Self {
        let mut entries = entries;
        entries.sort_by(|left, right| left.0.cmp(&right.0));

        let keys = entries.iter().map(|(key, _)| key.clone()).collect();
        let hash = hash_env_entries(&entries);

        Self::All { keys, hash }
    }

    /// Build one key from whitelisted environment variables.
    pub fn whitelist(keys: &[String], mut value_for: impl FnMut(&str) -> Option<String>) -> Self {
        let keys = normalize_profile_keys(keys.to_vec());
        let mut entries = Vec::with_capacity(keys.len());

        for key in &keys {
            let value = value_for(key);
            entries.push((key.clone(), value));
        }

        let hash = hash_optional_env_entries(&entries);

        Self::Whitelist { keys, hash }
    }

    /// Return the environment variable names included in this identity.
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
fn hash_env_entries(entries: &[(String, String)]) -> u128 {
    let mut hasher = StableHasher::new();
    for (key, value) in entries {
        hasher.update_len_prefixed(key.as_bytes());
        hasher.update(&[1]);
        hasher.update_len_prefixed(value.as_bytes());
    }

    hasher.finish_u128()
}

/// Hash optional environment entries by preserving absent values.
fn hash_optional_env_entries(entries: &[(String, Option<String>)]) -> u128 {
    let mut hasher = StableHasher::new();
    for (key, value) in entries {
        hasher.update_len_prefixed(key.as_bytes());
        match value {
            Some(value) => {
                hasher.update(&[1]);
                hasher.update_len_prefixed(value.as_bytes());
            }
            None => hasher.update(&[0]),
        }
    }

    hasher.finish_u128()
}
