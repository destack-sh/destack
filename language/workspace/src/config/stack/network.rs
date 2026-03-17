use serde::Deserialize;

use super::common::{StackTlsMode, StackTlsModeJson};

/// Stack network configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackNetworkOptions {
    /// Allowed callback origins or domains.
    pub callbacks: Vec<String>,
    /// Trusted origins outside the browser CORS model.
    pub trusted_origins: Vec<String>,
    /// TLS mode.
    pub tls_mode: Option<StackTlsMode>,
    /// Egress policy.
    pub egress: StackEgressOptions,
}

impl StackNetworkOptions {
    /// Inherit unset network settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.callbacks.is_empty() {
            self.callbacks = parent.callbacks.clone();
        }
        if self.trusted_origins.is_empty() {
            self.trusted_origins = parent.trusted_origins.clone();
        }
        if self.tls_mode.is_none() {
            self.tls_mode = parent.tls_mode;
        }

        self.egress.extend_from(&parent.egress);
    }
}

impl From<&StackNetworkJson> for StackNetworkOptions {
    fn from(json: &StackNetworkJson) -> Self {
        Self {
            callbacks: json.callbacks.clone().unwrap_or_default(),
            trusted_origins: json.trusted_origins.clone().unwrap_or_default(),
            tls_mode: json.tls_mode.map(StackTlsMode::from),
            egress: StackEgressOptions::from(&json.egress),
        }
    }
}

/// Egress policy options.
#[derive(Debug, Clone, Default)]
pub struct StackEgressOptions {
    /// Allowed egress labels or destinations.
    pub allow: Vec<String>,
    /// Denied egress labels or destinations.
    pub deny: Vec<String>,
}

impl StackEgressOptions {
    /// Inherit unset egress settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.allow.is_empty() {
            self.allow = parent.allow.clone();
        }
        if self.deny.is_empty() {
            self.deny = parent.deny.clone();
        }
    }
}

impl From<&StackEgressJson> for StackEgressOptions {
    fn from(json: &StackEgressJson) -> Self {
        Self {
            allow: json.allow.clone().unwrap_or_default(),
            deny: json.deny.clone().unwrap_or_default(),
        }
    }
}

/// Internal trust and egress policy.
///
/// Inputs: callbacks, trusted origins, TLS defaults, and egress rules.
/// Outputs: connectivity constraints for platform lowering.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackNetworkJson {
    /// Allowed callback origins or domains.
    pub callbacks: Option<Vec<String>>,
    /// Trusted origins outside the browser CORS model.
    pub trusted_origins: Option<Vec<String>>,
    /// TLS mode.
    pub tls_mode: Option<StackTlsModeJson>,
    /// Egress policy.
    #[serde(default)]
    pub egress: StackEgressJson,
}

/// Egress policy JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackEgressJson {
    /// Allowed egress labels or destinations.
    pub allow: Option<Vec<String>>,
    /// Denied egress labels or destinations.
    pub deny: Option<Vec<String>>,
}
