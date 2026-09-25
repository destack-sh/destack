use std::hash::Hash;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use tspp_core::StableHasher;

use crate::{ConditionSet, EnvironmentKey, Output, Stability, TargetAbi, TargetArch, TargetVendor};

const PROFILE_ID_DOMAIN: &[u8] = b"profile";

/// Diagnostic policy selected by a compiler profile.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticPolicy {
    /// Allow without diagnostics.
    #[default]
    Allow,
    /// Allow with a warning.
    Warn,
    /// Forbid with an error.
    Deny,
}

/// Canonical profile key for semantic identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ProfileKey {
    /// Output selected for the profile.
    pub output: Output,
    /// Stability promised by the selected package or product.
    pub stability: Option<Stability>,
    /// Active source graph and runtime conditions for the profile.
    pub conditions: ConditionSet,
    /// Target architecture for the profile.
    pub architecture: Option<TargetArch>,
    /// Target vendor for the profile.
    pub vendor: Option<TargetVendor>,
    /// Target application binary interface for the profile.
    pub abi: Option<TargetAbi>,
    /// Normalized global provider roots for the profile.
    pub globals: Vec<String>,
    /// Default tree tag builder provider.
    pub tree: Option<String>,
    /// Automatically derived interfaces for the profile.
    pub derive: Vec<String>,
    /// Compile-time environment identity for `import.meta.env`.
    pub env: EnvironmentKey,
    /// Policy for managed memory features.
    pub no_managed: DiagnosticPolicy,
    /// Policy for heap allocation.
    pub no_heap: DiagnosticPolicy,
    /// Policy for runtime features.
    pub no_runtime: DiagnosticPolicy,
    /// Policy for dynamic dispatch.
    pub no_dynamic_dispatch: DiagnosticPolicy,
    /// Policy for unsafe operations.
    pub no_unsafe: DiagnosticPolicy,
    /// Policy for runtime reflection.
    pub no_reflection: DiagnosticPolicy,
    /// Policy for unwinding.
    pub no_unwind: DiagnosticPolicy,
    /// Policy for aliasing mutable borrows.
    pub no_aliasing_mutable_borrows: DiagnosticPolicy,
    /// Policy for implicit method receivers.
    pub no_implicit_receivers: DiagnosticPolicy,
}

impl ProfileKey {
    /// Hash this profile key into one stable cache identity.
    pub fn stable_hash(&self) -> u128 {
        let mut hasher = StableHasher::new();

        hasher.update_len_prefixed(PROFILE_ID_DOMAIN);
        self.hash(&mut hasher);

        hasher.finish_u128()
    }
}
