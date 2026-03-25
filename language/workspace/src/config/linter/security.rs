use serde::Deserialize;

use super::LinterOptions;

/// Security-category linter options.
#[derive(Debug, Clone)]
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

/// Security-category linter JSON options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterSecurityJson {
    /// Allow `rel="noreferrer"` without `noopener` in `no-blank-target`.
    pub no_blank_target_allow_no_referrer: Option<bool>,
    /// Domains allowed to use `target="_blank"` without rel hardening.
    pub no_blank_target_allow_domains: Option<Vec<String>>,
    /// Entropy threshold in tenths for `no-secrets`.
    pub no_secrets_entropy_threshold: Option<u32>,
}

impl LinterSecurityJson {
    /// Validate security-category configuration values.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    /// Apply security-category options to one linter options struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(no_blank_target_allow_no_referrer) = self.no_blank_target_allow_no_referrer {
            options.security.no_blank_target_allow_no_referrer = no_blank_target_allow_no_referrer;
        }

        if let Some(ref no_blank_target_allow_domains) = self.no_blank_target_allow_domains {
            options.security.no_blank_target_allow_domains = no_blank_target_allow_domains.clone();
        }

        if let Some(no_secrets_entropy_threshold) = self.no_secrets_entropy_threshold {
            options.security.no_secrets_entropy_threshold = no_secrets_entropy_threshold;
        }
    }
}
