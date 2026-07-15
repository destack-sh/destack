use destack_serde::Reflect;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// One lint diagnostic category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum LintCategory {
    /// Likely bugs and logic errors.
    Correctness,
    /// Code that is likely unintentional.
    Suspicious,
    /// Inefficient code.
    Performance,
    /// Canonical language style.
    Style,
    /// Potential vulnerabilities.
    Security,
}

impl LintCategory {
    /// Return the category letter used in lint diagnostic codes.
    pub const fn letter(self) -> char {
        match self {
            Self::Correctness => 'C',
            Self::Suspicious => 'U',
            Self::Performance => 'P',
            Self::Style => 'Y',
            Self::Security => 'S',
        }
    }

    /// Return the category name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Suspicious => "suspicious",
            Self::Performance => "performance",
            Self::Style => "style",
            Self::Security => "security",
        }
    }

    /// Return the category description.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Correctness => "detects likely bugs and logic errors",
            Self::Suspicious => "detects potentially unintentional code",
            Self::Performance => "detects inefficient code",
            Self::Style => "enforces canonical language style",
            Self::Security => "detects potential vulnerabilities",
        }
    }

    /// All lint categories.
    pub const ALL: &'static [Self] = &[
        Self::Correctness,
        Self::Suspicious,
        Self::Performance,
        Self::Style,
        Self::Security,
    ];
}

impl std::fmt::Display for LintCategory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Linter configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LinterOptions {
    /// Whether linting is enabled.
    pub enabled: bool,
    /// Explicit levels keyed by rule id or diagnostic code.
    pub rules: IndexMap<String, LintLevel>,
}

impl Default for LinterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
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
    /// Return whether this level enables the rule.
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Serialize the complete default linter configuration.
    #[test]
    fn test_serialize_default_linter_options() {
        let actual = serde_json::to_value(LinterOptions::default()).unwrap();
        let expected = json!({
            "enabled": true,
            "rules": {},
        });

        assert_eq!(actual, expected);
    }

    /// Round trip every supported lint level with its canonical spelling.
    #[test]
    fn test_roundtrip_lint_levels() {
        let expected = json!({
            "enabled": true,
            "rules": {
                "allow-rule": "off",
                "warn-rule": "warning",
                "deny-rule": "error",
            },
        });
        let options: LinterOptions = serde_json::from_value(expected.clone()).unwrap();
        let actual = serde_json::to_value(options).unwrap();

        assert_eq!(actual, expected);
    }

    /// Reject obsolete or misspelled linter fields.
    #[test]
    fn test_reject_unknown_linter_fields() {
        let result = serde_json::from_value::<LinterOptions>(json!({
            "enabled": true,
            "overrides": {},
        }));

        assert!(result.is_err());
    }
}
