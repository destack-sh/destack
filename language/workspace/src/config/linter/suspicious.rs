use serde::Deserialize;

use super::LinterOptions;

/// Suspicious-category linter options.
#[derive(Debug, Clone, Default)]
pub struct LinterSuspiciousOptions {}

/// Suspicious-category linter JSON options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterSuspiciousJson {}

impl LinterSuspiciousJson {
    /// Validate suspicious-category configuration values.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    /// Apply suspicious-category options to one linter options struct.
    pub fn apply(&self, _options: &mut LinterOptions) {}
}
