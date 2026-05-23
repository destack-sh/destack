use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Configuration patch applied to one config value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigPatch {
    /// Patch path (e.g. compiler.target).
    pub path: String,
    /// Patch payload value.
    pub value: Value,
}

impl ConfigPatch {
    /// Return the validated path segments for this patch.
    pub fn path_segments(&self) -> Result<Vec<&str>, String> {
        let segments = self.path.split('.').collect::<Vec<_>>();

        if segments.is_empty() {
            return Err("empty config patch path".to_string());
        }

        if segments.iter().any(|segment| segment.is_empty()) {
            return Err(format!("invalid config patch path: {}", self.path));
        }

        Ok(segments)
    }
}

/// Apply one list of config patches to one json value.
pub fn apply_config_patches_to_json(
    json: &mut Value,
    patches: &[ConfigPatch],
) -> Result<(), String> {
    for patch in patches {
        let path = patch.path_segments()?;
        apply_config_patch_to_json(json, &path, &patch.value);
    }

    Ok(())
}

/// Apply one config patch to one json path.
fn apply_config_patch_to_json(target: &mut Value, path: &[&str], value: &Value) {
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

    apply_config_patch_to_json(child, &path[1..], value);
}

/// Merge one patch value into one json value.
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
