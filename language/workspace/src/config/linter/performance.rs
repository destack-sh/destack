use serde::Deserialize;

use super::{LinterOptions, UnicodeRegexpRequireFlag, UnicodeRegexpRequireFlagJson};

/// Performance-category linter options.
#[derive(Debug, Clone)]
pub struct LinterPerformanceOptions {
    /// Required Unicode regex flag for `require-unicode-regexp`.
    pub require_unicode_regexp_require_flag: UnicodeRegexpRequireFlag,
}

impl Default for LinterPerformanceOptions {
    fn default() -> Self {
        Self {
            require_unicode_regexp_require_flag: UnicodeRegexpRequireFlag::default(),
        }
    }
}

/// Performance-category linter JSON options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterPerformanceJson {
    /// Required Unicode regex flag for `require-unicode-regexp`.
    pub require_unicode_regexp_require_flag: Option<UnicodeRegexpRequireFlagJson>,
}

impl LinterPerformanceJson {
    /// Validate performance-category configuration values.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    /// Apply performance-category options to one linter options struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(require_unicode_regexp_require_flag) = self.require_unicode_regexp_require_flag
        {
            options.performance.require_unicode_regexp_require_flag =
                require_unicode_regexp_require_flag.into();
        }
    }
}
