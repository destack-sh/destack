use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Invocation override applied to one manifest value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestOverride {
    /// Manifest path, such as `compiler.target`.
    pub path: String,
    /// Override payload value.
    pub value: Value,
}

impl ManifestOverride {
    /// Return the validated path segments for this override.
    pub fn path_segments(&self) -> Result<Vec<&str>, String> {
        // reject empty paths
        if self.path.is_empty() {
            return Err("empty manifest override path".to_string());
        }

        // split path
        let segments = self.path.split('.').collect::<Vec<_>>();

        // reject empty path segments
        if segments.iter().any(|segment| segment.is_empty()) {
            return Err(format!("invalid manifest override path: {}", self.path));
        }

        Ok(segments)
    }
}

/// Apply one list of manifest overrides to one JSON value.
pub fn apply_manifest_overrides_to_json(
    json: &mut Value,
    overrides: &[ManifestOverride],
) -> Result<(), String> {
    // apply overrides in order
    for override_ in overrides {
        let path = override_.path_segments()?;
        apply_manifest_override_to_json(json, &path, &override_.value);
    }

    Ok(())
}

/// Apply one manifest override to one JSON path.
fn apply_manifest_override_to_json(target: &mut Value, path: &[&str], value: &Value) {
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

    // descend into child object
    let child = object
        .entry(path[0].to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    apply_manifest_override_to_json(child, &path[1..], value);
}

/// Merge one override value into one JSON value.
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

/// Ensure one JSON value is an object.
fn ensure_json_object(value: &mut Value) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
}
