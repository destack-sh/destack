use destack_core::stable_hash_key_value_128;
use serde::{Deserialize, Serialize};

use crate::{
    EmitFormat, Host, HostEnvironmentKey, Platform, Runtime, TargetAbi, TargetArch, TargetVendor,
    normalize_profile_keys,
};

const PROFILE_ID_DOMAIN: &[u8] = b"profile";

/// Flags that affect profile identity.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub struct ProfileFlags {
    /// Forbid managed memory features.
    pub no_managed: bool,
    /// Forbid heap allocation.
    pub no_heap: bool,
    /// Forbid runtime features.
    pub no_runtime: bool,
    /// Forbid internal protocol imports.
    pub no_internal_import: bool,
    /// Require static dispatch.
    pub no_implicit_dynamic_dispatch: bool,
    /// Forbid `throw`.
    pub no_throw: bool,
}

/// Canonical profile key for semantic identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProfileKey {
    /// Emit format for the profile.
    pub emit: EmitFormat,
    /// Runtime environment for the profile.
    pub runtime: Runtime,
    /// Target platform for the profile.
    pub platform: Platform,
    /// Target host environment for the profile.
    pub host: Host,
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
    /// Active source graph modes for the profile.
    pub modes: Vec<String>,
    /// Compile-time environment identity for `import.meta.env`.
    pub env: HostEnvironmentKey,
    /// Flags that affect semantic behavior.
    pub flags: ProfileFlags,
}

#[allow(clippy::too_many_arguments)]
impl ProfileKey {
    /// Create a profile key with normalized global entries.
    pub fn new(
        emit: EmitFormat,
        runtime: Runtime,
        platform: Platform,
        host: Host,
        target_arch: Option<TargetArch>,
        target_vendor: Option<TargetVendor>,
        target_abi: Option<TargetAbi>,
        globals: Vec<String>,
        tree: Option<String>,
        derive: Vec<String>,
        modes: Vec<String>,
        env: HostEnvironmentKey,
        flags: ProfileFlags,
    ) -> Self {
        let globals = normalize_profile_keys(globals);
        let derive = normalize_profile_keys(derive);
        let modes = normalize_mode_keys(modes);

        Self {
            emit,
            runtime,
            platform,
            host,
            target_arch,
            target_vendor,
            target_abi,
            globals,
            tree,
            derive,
            modes,
            env,
            flags,
        }
    }

    /// Hash this profile key into one stable cache identity.
    pub fn stable_hash(&self) -> u128 {
        let bytes = postcard::to_allocvec(self).unwrap_or_else(|error| {
            panic!("failed to serialize profile key for stable hashing: {error}")
        });

        stable_hash_key_value_128(PROFILE_ID_DOMAIN, &bytes)
    }
}

/// Normalize mode keys without changing active mode order.
fn normalize_mode_keys(modes: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(modes.len());

    for mode in modes {
        if !normalized.iter().any(|known| known == &mode) {
            normalized.push(mode);
        }
    }

    normalized
}
