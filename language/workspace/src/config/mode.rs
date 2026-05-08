use std::collections::HashSet;

use indexmap::IndexMap;
use serde::Deserialize;

/// Development mode name.
pub const MODE_DEV: &str = "dev";

/// Production mode name.
pub const MODE_PROD: &str = "prod";

/// Test mode name.
pub const MODE_TEST: &str = "test";

/// Benchmark mode name.
pub const MODE_BENCH: &str = "bench";

/// Lint mode name.
pub const MODE_LINT: &str = "lint";

/// Well-known mode names with built-in command and file suffix behavior.
pub const BUILTIN_MODES: &[&str] = &[MODE_DEV, MODE_PROD, MODE_TEST, MODE_BENCH, MODE_LINT];

/// Named source graph mode options.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ModeOptions {
    /// Mode names inherited before this mode.
    pub extends: Vec<String>,
}

impl ModeOptions {
    /// Convert from one JSON mode.
    pub fn from_json(json: &ModeJson) -> Self {
        Self {
            extends: json
                .extends
                .as_ref()
                .map(ModeExtends::names)
                .unwrap_or_default(),
        }
    }
}

/// Return the built-in options for one mode.
pub fn builtin_mode_options(mode: &str) -> Option<ModeOptions> {
    if !BUILTIN_MODES.contains(&mode) {
        return None;
    }

    Some(ModeOptions::default())
}

/// Return whether one mode name is well-known to the toolchain.
pub fn is_builtin_mode(mode: &str) -> bool {
    builtin_mode_options(mode).is_some()
}

/// Source graph mode JSON from `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ModeJson {
    /// Mode names inherited before this mode.
    pub extends: Option<ModeExtends>,
}

/// Mode extends field from `destack.json`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ModeExtends {
    /// Extend one mode.
    One(String),
    /// Extend many modes in order.
    Many(Vec<String>),
}

impl ModeExtends {
    /// Return the referenced mode names.
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::One(mode) => vec![mode.clone()],
            Self::Many(modes) => modes.clone(),
        }
    }
}

/// Validate that mode inheritance is closed and acyclic.
pub fn validate_modes(modes: &IndexMap<String, ModeOptions>) -> Result<(), String> {
    let mut visited = HashSet::new();
    let mut active = Vec::new();

    // validate every declared mode root
    for mode in modes.keys() {
        validate_mode(mode, modes, &mut visited, &mut active)?;
    }

    Ok(())
}

fn validate_mode(
    mode: &str,
    modes: &IndexMap<String, ModeOptions>,
    visited: &mut HashSet<String>,
    active: &mut Vec<String>,
) -> Result<(), String> {
    // skip modes already proven valid
    if visited.contains(mode) {
        return Ok(());
    }

    // reject recursive inheritance
    if active.iter().any(|active_mode| active_mode == mode) {
        let mut cycle = active.clone();
        cycle.push(mode.to_string());
        return Err(format!("mode inheritance cycle: {}", cycle.join(" -> ")));
    }

    let builtin_options;
    let options = if let Some(options) = modes.get(mode) {
        options
    } else if let Some(options) = builtin_mode_options(mode) {
        builtin_options = options;
        &builtin_options
    } else {
        return Err(format!("unknown mode '{mode}'"));
    };

    active.push(mode.to_string());

    // parents are activated before the child mode
    for parent in &options.extends {
        validate_mode(parent, modes, visited, active)?;
    }

    active.pop();
    visited.insert(mode.to_string());

    Ok(())
}
