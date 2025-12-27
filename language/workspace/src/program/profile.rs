use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;

use crate::{DsConfigCompilerOptions, Platform, Runtime};

/// Unique identifier for profiles.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProfileId(pub u32);

impl std::fmt::Debug for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl ProfileId {
    /// Wrap an id as a ProfileId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Comptime environment snapshot used for profile identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnvSnapshot {
    /// Represents a full environment snapshot with keys and hashed values.
    All { keys: Vec<String>, hash: u64 },
    /// Represents a whitelisted environment snapshot with keys and hashed values.
    Whitelist { keys: Vec<String>, hash: u64 },
}

/// Flags that affect profile identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
    /// Require explicit `self.` in methods.
    pub no_implicit_self: bool,
    /// Forbid `arguments` usage.
    pub no_arguments: bool,
    /// Forbid redeclaration of locals.
    pub no_redeclared_locals: bool,
    /// Require explicit managed types.
    pub no_implicit_managed_type: bool,
    /// Require explicit managed values.
    pub no_implicit_managed_value: bool,
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
}

/// Canonical profile key for semantic identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileKey {
    /// Runtime environment for the profile.
    pub runtime: Runtime,
    /// Target platform for the profile.
    pub platform: Platform,
    /// Normalized library set for the profile.
    pub lib: Vec<String>,
    /// Debug flag exposed to `import.meta`.
    pub debug: bool,
    /// Test flag exposed to `import.meta`.
    pub test: bool,
    /// Comptime environment snapshot for `import.meta.env`.
    pub env: EnvSnapshot,
    /// Flags that affect semantic behavior.
    pub flags: ProfileFlags,
}

/// A registered profile.
#[derive(Debug, Clone)]
pub struct Profile {
    /// The profile id.
    pub id: ProfileId,
    /// The canonical key for the profile.
    pub key: ProfileKey,
}

/// Registry for profiles keyed by ProfileKey.
#[derive(Debug)]
pub struct ProfileRegistry {
    /// Next profile id to assign.
    next_profile_id: AtomicU32,
    /// Mapping from profile key to id.
    profile_by_key: DashMap<ProfileKey, ProfileId>,
    /// Mapping from profile id to profile data.
    profile_by_id: DashMap<ProfileId, Profile>,
}

impl Default for ProfileRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileRegistry {
    /// Create a new profile registry.
    pub fn new() -> Self {
        Self {
            next_profile_id: AtomicU32::new(1),
            profile_by_key: DashMap::new(),
            profile_by_id: DashMap::new(),
        }
    }

    /// Get or create a profile id for the given key.
    pub fn get_or_create(&self, key: ProfileKey) -> ProfileId {
        if let Some(existing) = self.profile_by_key.get(&key) {
            return *existing;
        }

        let id = ProfileId::new(self.next_profile_id.fetch_add(1, Ordering::Relaxed));
        let entry = self.profile_by_key.entry(key.clone()).or_insert(id);
        if *entry != id {
            return *entry;
        }

        self.profile_by_id.insert(id, Profile { id, key });
        id
    }

    /// Get a profile by id.
    pub fn get(&self, id: ProfileId) -> Option<Profile> {
        self.profile_by_id.get(&id).map(|entry| entry.clone())
    }
}

impl EnvSnapshot {
    /// Snapshot all environment keys and values.
    pub fn from_env_all() -> Self {
        let mut entries: Vec<(String, String)> = std::env::vars().collect();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let keys = entries.iter().map(|(key, _)| key.clone()).collect();
        let hash = hash_env_entries(&entries);
        Self::All { keys, hash }
    }

    /// Snapshot only whitelisted environment keys and values.
    pub fn from_env_whitelist(keys: &[String]) -> Self {
        let keys = normalize_keys(keys.to_vec());
        let mut entries = Vec::with_capacity(keys.len());
        for key in &keys {
            let value = std::env::var(key).unwrap_or_default();
            entries.push((key.clone(), value));
        }
        let hash = hash_env_entries(&entries);
        Self::Whitelist { keys, hash }
    }

    /// Return the environment keys included in this snapshot.
    pub fn keys(&self) -> &[String] {
        match self {
            Self::All { keys, .. } | Self::Whitelist { keys, .. } => keys,
        }
    }
}

impl From<&DsConfigCompilerOptions> for ProfileFlags {
    fn from(options: &DsConfigCompilerOptions) -> Self {
        Self {
            no_any: options.no_any,
            no_unknown: options.no_unknown,
            no_imprecise_primitives: options.no_imprecise_primitives,
            no_implicit_conversions: options.no_implicit_conversions,
            no_unsafe_type_assertions: options.no_unsafe_type_assertions,
            no_implicit_self: options.no_implicit_self,
            no_arguments: options.no_arguments,
            no_redeclared_locals: options.no_redeclared_locals,
            no_implicit_managed_type: options.no_implicit_managed_type,
            no_implicit_managed_value: options.no_implicit_managed_value,
            no_managed: options.no_managed,
            no_runtime: options.no_runtime,
            no_referential_equality: options.no_referential_equality,
            no_dynamic_evaluation: options.no_dynamic_evaluation,
            no_global_this: options.no_global_this,
            no_dynamic_import: options.no_dynamic_import,
            no_dynamic_shapes: options.no_dynamic_shapes,
            no_computed_property_access: options.no_computed_property_access,
            no_proxy: options.no_proxy,
            no_implicit_dynamic_dispatch: options.no_implicit_dynamic_dispatch,
            no_exceptions: options.no_exceptions,
        }
    }
}

impl ProfileKey {
    /// Create a profile key with normalized library entries.
    pub fn new(
        runtime: Runtime,
        platform: Platform,
        lib: Vec<String>,
        debug: bool,
        test: bool,
        env: EnvSnapshot,
        flags: ProfileFlags,
    ) -> Self {
        let lib = normalize_keys(lib);
        Self {
            runtime,
            platform,
            lib,
            debug,
            test,
            env,
            flags,
        }
    }
}

/// Normalize a list of keys by sorting and deduplicating.
fn normalize_keys(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys.dedup();
    keys
}

/// Hash a list of environment entries by hashing the key and value.
fn hash_env_entries(entries: &[(String, String)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (key, value) in entries {
        key.hash(&mut hasher);
        value.hash(&mut hasher);
    }
    hasher.finish()
}
