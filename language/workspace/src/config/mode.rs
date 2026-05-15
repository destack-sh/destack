use indexmap::IndexMap;
use serde::Deserialize;

use super::{
    DependencyJsonMap, DependencyMap, dependency_options_from_json, validate_dependency_json_map,
};

/// Built-in source graph mode declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mode {
    /// Stable mode name.
    pub name: &'static str,
    /// Human-readable mode description.
    pub description: &'static str,
}

impl Mode {
    /// Development source graph mode.
    pub const DEV: Self = Self {
        name: "dev",
        description: "Development mode.",
    };

    /// Debug source graph mode.
    pub const DEBUG: Self = Self {
        name: "debug",
        description: "Debug mode.",
    };

    /// Production source graph mode.
    pub const PROD: Self = Self {
        name: "prod",
        description: "Production mode.",
    };

    /// Test source graph mode.
    pub const TEST: Self = Self {
        name: "test",
        description: "Test mode.",
    };

    /// Benchmark source graph mode.
    pub const BENCH: Self = Self {
        name: "bench",
        description: "Benchmark mode.",
    };

    /// Fuzzing source graph mode.
    pub const FUZZ: Self = Self {
        name: "fuzz",
        description: "Fuzzing mode.",
    };

    /// Simulation source graph mode.
    pub const SIM: Self = Self {
        name: "sim",
        description: "Simulation mode.",
    };

    /// Lint source graph mode.
    pub const LINT: Self = Self {
        name: "lint",
        description: "Lint mode.",
    };

    /// Built-in source graph modes.
    pub const BUILTINS: &'static [Self] = &[
        Self::DEV,
        Self::DEBUG,
        Self::PROD,
        Self::TEST,
        Self::BENCH,
        Self::FUZZ,
        Self::SIM,
        Self::LINT,
    ];

    /// Return normalized options for this built-in mode.
    pub fn options(self) -> ModeOptions {
        ModeOptions {
            description: Some(self.description.to_string()),
            ..ModeOptions::default()
        }
    }
}

/// Named source graph mode options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModeOptions {
    /// Human-readable mode description.
    pub description: Option<String>,
    /// Mode labels.
    pub labels: IndexMap<String, String>,
    /// Mode names included before this mode.
    pub extends: Vec<String>,
    /// Dependencies enabled by this mode.
    pub dependencies: DependencyMap,
}

impl ModeOptions {
    /// Convert from one JSON mode.
    pub fn from_json(json: &ModeJson) -> Self {
        Self {
            description: json.description.clone(),
            labels: json.labels.clone().unwrap_or_default(),
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
    Mode::BUILTINS
        .iter()
        .map(|mode| (mode.name.to_string(), mode.options()))
        .collect()
}

/// Source graph mode JSON from `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ModeJson {
    /// Human-readable mode description.
    pub description: Option<String>,
    /// Mode labels.
    pub labels: Option<IndexMap<String, String>>,
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
