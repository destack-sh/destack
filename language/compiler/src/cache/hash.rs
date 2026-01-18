use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;
use serde_json::Value;

pub(super) const DSCONFIG_CACHE_IGNORED_KEYS: [&str; 6] = [
    "cache",
    "watch",
    "formatter",
    "linter",
    "targets",
    "defaultTarget",
];

/// Hash bytes with a stable hasher.
pub(super) fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Hash a JSON object with a stable key ordering.
fn hash_json_object(map: &serde_json::Map<String, Value>, hasher: &mut FxHasher) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        key.hash(hasher);
        if let Some(value) = map.get(key) {
            hash_json_value_inner(value, hasher);
        }
    }
}

/// Hash a JSON value with canonical ordering for object keys.
fn hash_json_value_inner(value: &Value, hasher: &mut FxHasher) {
    match value {
        Value::Null => {
            0_u8.hash(hasher);
        }
        Value::Bool(value) => {
            1_u8.hash(hasher);
            value.hash(hasher);
        }
        Value::Number(value) => {
            2_u8.hash(hasher);
            value.to_string().hash(hasher);
        }
        Value::String(value) => {
            3_u8.hash(hasher);
            value.hash(hasher);
        }
        Value::Array(values) => {
            4_u8.hash(hasher);
            values.len().hash(hasher);
            for entry in values {
                hash_json_value_inner(entry, hasher);
            }
        }
        Value::Object(map) => {
            5_u8.hash(hasher);
            hash_json_object(map, hasher);
        }
    }
}

/// Hash a JSON value with stable ordering for cache purposes.
pub(super) fn hash_json_value(value: &Value) -> u64 {
    let mut hasher = FxHasher::default();
    hash_json_value_inner(value, &mut hasher);
    hasher.finish()
}

/// Trim ignored keys from a JSON object value.
pub(super) fn trim_json_object(value: &Value, ignored_keys: &[&str]) -> Value {
    let Value::Object(map) = value else {
        return value.clone();
    };

    let mut trimmed = serde_json::Map::with_capacity(map.len());
    for (key, value) in map {
        if ignored_keys.contains(&key.as_str()) {
            continue;
        }
        trimmed.insert(key.clone(), value.clone());
    }

    Value::Object(trimmed)
}

#[cfg(test)]
mod tests {
    use super::{DSCONFIG_CACHE_IGNORED_KEYS, hash_json_value, trim_json_object};
    use serde_json::json;

    /// Hashing ignores key order for cache invalidation.
    #[test]
    fn test_json_hash_is_order_invariant() {
        let first = json!({
            "compilerOptions": { "strict": true, "noImplicitAny": true },
            "include": ["src"],
            "exclude": ["dist"],
        });
        let second = json!({
            "exclude": ["dist"],
            "include": ["src"],
            "compilerOptions": { "noImplicitAny": true, "strict": true },
        });

        // assert hashes are stable across key order
        assert_eq!(hash_json_value(&first), hash_json_value(&second));
    }

    /// Hashing changes when JSON values change.
    #[test]
    fn test_json_hash_changes_on_value_change() {
        let first = json!({
            "compilerOptions": { "strict": true },
            "include": ["src"],
        });
        let second = json!({
            "compilerOptions": { "strict": false },
            "include": ["src"],
        });

        // assert hashes diverge for semantic changes
        assert_ne!(hash_json_value(&first), hash_json_value(&second));
    }

    /// Hashing ignores tooling only sections when trimmed.
    #[test]
    fn test_json_trim_ignores_tooling_sections() {
        let first = json!({
            "compilerOptions": { "strict": true },
            "cache": { "mode": "disk" },
            "watch": { "debounceMs": 10 },
            "formatter": { "lineWidth": 100 },
            "linter": { "preset": "recommended" },
        });
        let second = json!({
            "compilerOptions": { "strict": true },
            "cache": { "mode": "memory" },
            "watch": { "debounceMs": 50 },
            "formatter": { "lineWidth": 80 },
            "linter": { "preset": "strict" },
        });

        // assert hashes match despite tooling only changes
        let trimmed_first = trim_json_object(&first, &DSCONFIG_CACHE_IGNORED_KEYS);
        let trimmed_second = trim_json_object(&second, &DSCONFIG_CACHE_IGNORED_KEYS);
        assert_eq!(
            hash_json_value(&trimmed_first),
            hash_json_value(&trimmed_second)
        );
    }

    /// Hashing ignores key order for cache invalidation.
    #[test]
    fn test_tsconfig_hash_is_order_invariant() {
        let first = json!({
            "compilerOptions": { "strict": true, "target": "ES2022" },
            "include": ["src"],
            "exclude": ["dist"],
        });
        let second = json!({
            "exclude": ["dist"],
            "include": ["src"],
            "compilerOptions": { "target": "ES2022", "strict": true },
        });

        // assertion block
        assert_eq!(hash_json_value(&first), hash_json_value(&second));
    }

    /// Hashing changes when tsconfig values change.
    #[test]
    fn test_tsconfig_hash_changes_on_value_change() {
        let first = json!({
            "compilerOptions": { "strict": true },
            "include": ["src"],
        });
        let second = json!({
            "compilerOptions": { "strict": false },
            "include": ["src"],
        });

        // assertion block
        assert_ne!(hash_json_value(&first), hash_json_value(&second));
    }
}
