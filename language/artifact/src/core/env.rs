use destack_core::StableHasher;
use serde::{Deserialize, Serialize};

/// Comptime environment input used for profile identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentInput {
    /// Full environment snapshot with keys and hashed values.
    All { keys: Vec<String>, hash: u128 },
    /// Whitelisted environment snapshot with keys and hashed values.
    Whitelist { keys: Vec<String>, hash: u128 },
}

impl EnvironmentInput {
    /// Snapshot all provided environment keys and values.
    pub fn from_entries_all(entries: Vec<(String, String)>) -> Self {
        let mut entries = entries;
        entries.sort_by(|left, right| left.0.cmp(&right.0));

        let keys = entries.iter().map(|(key, _)| key.clone()).collect();
        let hash = hash_env_entries(&entries);

        Self::All { keys, hash }
    }

    /// Snapshot all environment keys and values.
    pub fn from_env_all() -> Self {
        let entries: Vec<(String, String)> = std::env::vars().collect();

        Self::from_entries_all(entries)
    }

    /// Snapshot only whitelisted environment keys and values from one value provider.
    pub fn from_values_whitelist(
        keys: &[String],
        mut value_for: impl FnMut(&str) -> Option<String>,
    ) -> Self {
        let keys = normalize_profile_keys(keys.to_vec());
        let mut entries = Vec::with_capacity(keys.len());

        for key in &keys {
            let value = value_for(key);
            entries.push((key.clone(), value));
        }

        let hash = hash_optional_env_entries(&entries);

        Self::Whitelist { keys, hash }
    }

    /// Snapshot only whitelisted environment keys and values.
    pub fn from_env_whitelist(keys: &[String]) -> Self {
        Self::from_values_whitelist(keys, |key| std::env::var(key).ok())
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
