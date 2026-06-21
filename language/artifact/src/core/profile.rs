use destack_serde::Schema;
use std::hash::Hash;

use destack_core::StableHasher;
use serde::{Deserialize, Serialize};

use crate::{ConditionSet, EmitFormat, EnvironmentKey, TargetAbi, TargetArch, TargetVendor};

const PROFILE_ID_DOMAIN: &[u8] = b"profile";

/// Canonical profile key for semantic identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
pub struct ProfileKey {
    /// Emit format for the profile.
    pub emit: EmitFormat,
    /// Active source graph and runtime conditions for the profile.
    pub conditions: ConditionSet,
    /// Target architecture for the profile.
    pub target_arch: Option<TargetArch>,
    /// Target vendor for the profile.
    pub target_vendor: Option<TargetVendor>,
    /// Target environment for the profile.
    pub target_abi: Option<TargetAbi>,
    /// Normalized global provider roots for the profile.
    pub globals: Vec<String>,
    /// Default tree tag builder provider.
    pub tree: Option<String>,
    /// Auto derive providers for the profile.
    pub derive: Vec<String>,
    /// Compile-time environment identity for `import.meta.env`.
    pub env: EnvironmentKey,
    /// Forbid managed memory features.
    pub no_managed: bool,
    /// Forbid heap allocation.
    pub no_heap: bool,
    /// Forbid runtime features.
    pub no_runtime: bool,
    /// Require static dispatch.
    pub no_dynamic_dispatch: bool,
    /// Forbid unsafe operations.
    pub no_unsafe: bool,
    /// Forbid runtime reflection.
    pub no_reflection: bool,
    /// Forbid unwinding.
    pub no_unwind: bool,
    /// Forbid aliasing mutable borrows.
    pub no_aliasing_mutable_borrows: bool,
    /// Forbid implicit method receivers.
    pub no_implicit_receivers: bool,
    /// Emit checked type sidecars.
    pub emit_checked_types: bool,
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
