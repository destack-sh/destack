use indexmap::IndexMap;
use serde::Deserialize;

use super::{
    DependencyJsonMap, DependencyMap, dependency_options_from_json, validate_dependency_json_map,
};

/// Development mode name.
pub const MODE_DEV: &str = "dev";

/// Debug mode name.
pub const MODE_DEBUG: &str = "debug";

/// Production mode name.
pub const MODE_PROD: &str = "prod";

/// Test mode name.
pub const MODE_TEST: &str = "test";

/// Benchmark mode name.
pub const MODE_BENCH: &str = "bench";

/// Lint mode name.
pub const MODE_LINT: &str = "lint";

/// Named source graph mode options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModeOptions {
    /// Mode names included before this mode.
    pub extends: Vec<String>,
    /// Dependencies enabled by this mode.
    pub dependencies: DependencyMap,
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
            dependencies: dependency_options_from_json(&json.dependencies),
        }
    }
}

/// Return the built-in source graph modes.
pub fn builtin_modes() -> IndexMap<String, ModeOptions> {
    builtin_mode_names()
        .into_iter()
        .map(|mode| (mode.to_string(), ModeOptions::default()))
        .collect()
}

/// Return the built-in source graph mode names.
pub fn builtin_mode_names() -> &'static [&'static str] {
    &[
        MODE_DEV, MODE_DEBUG, MODE_PROD, MODE_TEST, MODE_BENCH, MODE_LINT,
    ]
}

/// Source graph mode JSON from `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ModeJson {
    /// Mode names included before this mode.
    pub extends: Option<ModeExtends>,
    /// Dependencies enabled by this mode.
    pub dependencies: Option<DependencyJsonMap>,
}

impl ModeJson {
    /// Validate one source graph mode declaration.
    pub fn validate(&self) -> Result<(), String> {
        validate_dependency_json_map(self.dependencies.as_ref())
    }
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
