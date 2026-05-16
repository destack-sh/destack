use serde::{Deserialize, Serialize};

use super::UnicodeRegexpRequireFlag;

/// Performance-category linter options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LinterPerformanceOptions {
    /// Required Unicode regex flag for `require-unicode-regexp`.
    pub require_unicode_regexp_require_flag: UnicodeRegexpRequireFlag,
}
