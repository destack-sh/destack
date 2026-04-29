use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Configuration override applied to one config value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigOverride {
    /// Override path (e.g. compiler.strict).
    pub path: String,
    /// Override payload value.
    pub value: Value,
}

impl ConfigOverride {
    /// Return the validated path segments for this override.
    pub fn path_segments(&self) -> Result<Vec<&str>, String> {
        let segments = self.path.split('.').collect::<Vec<_>>();

        if segments.is_empty() {
            return Err("empty config override path".to_string());
        }

        if segments.iter().any(|segment| segment.is_empty()) {
            return Err(format!("invalid config override path: {}", self.path));
        }

        Ok(segments)
    }
}

/// Apply one list of config overrides to one json value.
pub fn apply_config_overrides_to_json(
    json: &mut Value,
    overrides: &[ConfigOverride],
) -> Result<(), String> {
    for override_entry in overrides {
        let path = override_entry.path_segments()?;
        apply_config_override_to_json(json, &path, &override_entry.value);
    }

    Ok(())
}

/// Apply one config override to one json path.
fn apply_config_override_to_json(target: &mut Value, path: &[&str], value: &Value) {
    ensure_json_object(target);

    let Value::Object(object) = target else {
        return;
    };

    if path.len() == 1 {
        let entry = object
            .entry(path[0].to_string())
            .or_insert_with(|| Value::Object(Map::new()));

        merge_json_value(entry, value);
        return;
    }

    let child = object
        .entry(path[0].to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    apply_config_override_to_json(child, &path[1..], value);
}

/// Merge one override value into one json value.
fn merge_json_value(target: &mut Value, value: &Value) {
    match (target, value) {
        (Value::Object(target_object), Value::Object(value_object)) => {
            for (key, value) in value_object {
                let entry = target_object
                    .entry(key.clone())
                    .or_insert_with(|| Value::Object(Map::new()));
                merge_json_value(entry, value);
            }
        }
        (target, value) => *target = value.clone(),
    }
}

/// Ensure one json value is an object.
fn ensure_json_object(value: &mut Value) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{ConfigOverride, apply_config_overrides_to_json};

    /// Merge object overrides into existing config values.
    #[test]
    fn test_apply_config_overrides_to_json_merges_objects() {
        let mut json = json!({
            "formatter": {
                "lineWidth": 80,
                "quoteStyle": "double"
            }
        });
        let overrides = vec![ConfigOverride {
            path: "formatter".to_string(),
            value: json!({
                "lineWidth": 100
            }),
        }];

        apply_config_overrides_to_json(&mut json, &overrides).unwrap();

        assert_eq!(
            json,
            json!({
                "formatter": {
                    "lineWidth": 100,
                    "quoteStyle": "double"
                }
            })
        );
    }

    /// Reject override paths with empty segments.
    #[test]
    fn test_apply_config_overrides_to_json_rejects_empty_segments() {
        let mut json = Value::Object(Default::default());
        let overrides = vec![ConfigOverride {
            path: "formatter..lineWidth".to_string(),
            value: json!(100),
        }];

        assert_eq!(
            apply_config_overrides_to_json(&mut json, &overrides),
            Err("invalid config override path: formatter..lineWidth".to_string())
        );
    }
}
