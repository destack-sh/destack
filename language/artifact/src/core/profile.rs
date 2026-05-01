use destack_core::stable_hash_key_value_128;
use serde::{Deserialize, Serialize};

use crate::{
    EmitFormat, HostEnvironmentKey, Platform, Runtime, TargetAbi, TargetArch, TargetVendor,
    normalize_profile_keys,
};

const PROFILE_ID_DOMAIN: &[u8] = b"profile";

/// Flags that affect profile identity.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub struct ProfileFlags {
    /// Forbid use of `any`.
    pub no_any: bool,
    /// Forbid use of `unknown`.
    pub no_unknown: bool,
    /// Require precise primitive types.
    pub no_imprecise_primitives: bool,
    /// Require explicit conversions.
    pub no_implicit_conversions: bool,
    /// Forbid unsafe type assertions.
    pub no_unsafe_type_assertions: bool,
    /// Forbid must assertions.
    pub no_must_assertions: bool,
    /// Forbid definite assignment assertions.
    pub no_definite_assignment_assertions: bool,
    /// Forbid custom type guards.
    pub no_custom_type_guards: bool,
    /// Forbid unsound variance.
    pub no_unsound_variance: bool,
    /// Forbid unsound narrowing.
    pub no_unsound_narrowing: bool,
    /// Require deep readonly semantics.
    pub deep_readonly: bool,
    /// Forbid untrusted declaration files.
    pub no_untrusted_declarations: bool,
    /// Forbid redeclaration of locals.
    pub no_redeclared_locals: bool,
    /// Require explicit managed ownership.
    pub no_implicit_managed: bool,
    /// Forbid managed memory features.
    pub no_managed: bool,
    /// Forbid runtime features.
    pub no_runtime: bool,
    /// Forbid referential equality.
    pub no_referential_equality: bool,
    /// Forbid dynamic evaluation.
    pub no_dynamic_evaluation: bool,
    /// Forbid `globalThis`.
    pub no_global_this: bool,
    /// Forbid dynamic imports.
    pub no_dynamic_import: bool,
    /// Forbid internal protocol imports.
    pub no_internal_import: bool,
    /// Forbid dynamic shapes.
    pub no_dynamic_shapes: bool,
    /// Forbid computed property access.
    pub no_computed_property_access: bool,
    /// Forbid Proxy usage.
    pub no_proxy: bool,
    /// Require static dispatch.
    pub no_implicit_dynamic_dispatch: bool,
    /// Forbid exceptions.
    pub no_exceptions: bool,
    /// Enable strict builtin iterator return checking.
    pub strict_builtin_iterator_return: bool,
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
    /// Target architecture for the profile.
    pub target_arch: Option<TargetArch>,
    /// Target vendor for the profile.
    pub target_vendor: Option<TargetVendor>,
    /// Target environment for the profile.
    pub target_abi: Option<TargetAbi>,
    /// Normalized library set for the profile.
    pub lib: Vec<String>,
    /// Normalized global provider roots for the profile.
    pub globals: Vec<String>,
    /// Debug flag exposed to `import.meta`.
    pub debug: bool,
    /// Test flag exposed to `import.meta`.
    pub test: bool,
    /// Skip declaration diagnostics in compatibility mode.
    pub skip_lib_check: bool,
    /// Compile-time environment identity for `import.meta.env`.
    pub env: HostEnvironmentKey,
    /// Flags that affect semantic behavior.
    pub flags: ProfileFlags,
}

#[allow(clippy::too_many_arguments)]
impl ProfileKey {
    /// Create a profile key with normalized library and global entries.
    pub fn new(
        emit: EmitFormat,
        runtime: Runtime,
        platform: Platform,
        target_arch: Option<TargetArch>,
        target_vendor: Option<TargetVendor>,
        target_abi: Option<TargetAbi>,
        lib: Vec<String>,
        globals: Vec<String>,
        debug: bool,
        test: bool,
        skip_lib_check: bool,
        env: HostEnvironmentKey,
        flags: ProfileFlags,
    ) -> Self {
        let lib = normalize_profile_keys(lib);
        let globals = normalize_profile_keys(globals);

        Self {
            emit,
            runtime,
            platform,
            target_arch,
            target_vendor,
            target_abi,
            lib,
            globals,
            debug,
            test,
            skip_lib_check,
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
