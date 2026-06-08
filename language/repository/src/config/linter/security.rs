use serde::{Deserialize, Serialize};

/// Security-category linter options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LinterSecurityOptions {
    /// Allow `rel="noreferrer"` without `noopener` in `no-blank-target`.
    pub no_blank_target_allow_no_referrer: bool,
    /// Domains allowed to use `target="_blank"` without rel hardening.
    pub no_blank_target_allow_domains: Vec<String>,
    /// Entropy threshold in tenths for `no-secrets`.
    pub no_secrets_entropy_threshold: u32,
}

impl Default for LinterSecurityOptions {
    fn default() -> Self {
        Self {
            no_blank_target_allow_no_referrer: true,
            no_blank_target_allow_domains: Vec::new(),
            no_secrets_entropy_threshold: 41,
        }
    }
}
