use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;
use serde_json::Value;

const DSCONFIG_CACHE_IGNORED_KEYS: [&str; 6] = [
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
fn hash_json_object(
    map: &serde_json::Map<String, Value>,
    hasher: &mut FxHasher,
    filter_keys: Option<&[&str]>,
) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        if filter_keys
            .map(|filter| filter.contains(&key.as_str()))
            .unwrap_or(false)
        {
            continue;
        }

        key.hash(hasher);
        if let Some(value) = map.get(key) {
            hash_json_value(value, hasher);
        }
    }
}

/// Hash a JSON value with canonical ordering for object keys.
fn hash_json_value(value: &Value, hasher: &mut FxHasher) {
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
                hash_json_value(entry, hasher);
            }
        }
        Value::Object(map) => {
            5_u8.hash(hasher);
            hash_json_object(map, hasher, None);
        }
    }
}

/// Hash a dsconfig JSON value for cache purposes.
pub(super) fn hash_dsconfig_value(value: &Value) -> u64 {
    let mut hasher = FxHasher::default();
    match value {
        Value::Object(map) => {
            5_u8.hash(&mut hasher);
            hash_json_object(map, &mut hasher, Some(&DSCONFIG_CACHE_IGNORED_KEYS));
        }
        _ => hash_json_value(value, &mut hasher),
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::hash_dsconfig_value;
    use serde_json::json;

    /// Hashing ignores dsconfig key order for cache invalidation.
    #[test]
    fn test_dsconfig_hash_is_order_invariant() {
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
        assert_eq!(hash_dsconfig_value(&first), hash_dsconfig_value(&second));
    }

    /// Hashing changes when relevant config values change.
    #[test]
    fn test_dsconfig_hash_changes_on_value_change() {
        let first = json!({
            "compilerOptions": { "strict": true },
            "include": ["src"],
        });
        let second = json!({
            "compilerOptions": { "strict": false },
            "include": ["src"],
        });

        // assert hashes diverge for semantic changes
        assert_ne!(hash_dsconfig_value(&first), hash_dsconfig_value(&second));
    }

    /// Hashing ignores tooling only sections that do not affect compilation.
    #[test]
    fn test_dsconfig_hash_ignores_tooling_sections() {
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
        assert_eq!(hash_dsconfig_value(&first), hash_dsconfig_value(&second));
    }
}
