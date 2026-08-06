use destack_serde::Reflect;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Linter configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LinterOptions {
    /// Whether linting is enabled.
    pub enabled: bool,
    /// Lint ids selected exclusively.
    pub only: Vec<String>,
    /// Explicit levels keyed by lint id.
    pub rules: IndexMap<String, LintLevel>,
}

impl Default for LinterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            only: Vec::new(),
            rules: IndexMap::new(),
        }
    }
}

/// One configured lint level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum LintLevel {
    /// Disable the rule.
    Off,
    /// Report a warning.
    #[default]
    Warning,
    /// Report an error.
    Error,
}

impl LintLevel {
    /// Return the configuration spelling.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    /// Return whether this level enables the rule.
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }
}
