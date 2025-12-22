use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;

use crate::{OutputFormat, Platform, Runtime};

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
pub enum ComptimeEnvSnapshot {
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
    /// Output format for the profile.
    pub output: OutputFormat,
    /// Runtime environment for the profile.
    pub runtime: Runtime,
    /// Target platform for the profile.
    pub platform: Platform,
    /// Normalized library set for the profile.
    pub lib: Vec<String>,
    /// Debug flag exposed to `import.meta`.
    pub debug: bool,
    /// Comptime environment snapshot for `import.meta.env`.
    pub comptime_env: ComptimeEnvSnapshot,
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
    next_id: AtomicU32,
    /// Mapping from profile key to id.
    by_key: DashMap<ProfileKey, ProfileId>,
    /// Mapping from profile id to profile data.
    by_id: DashMap<ProfileId, Profile>,
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
            next_id: AtomicU32::new(1),
            by_key: DashMap::new(),
            by_id: DashMap::new(),
        }
    }

    /// Get or create a profile id for the given key.
    pub fn get_or_create(&self, key: ProfileKey) -> ProfileId {
        if let Some(existing) = self.by_key.get(&key) {
            return *existing;
        }

        let id = ProfileId::new(self.next_id.fetch_add(1, Ordering::Relaxed));
        let entry = self.by_key.entry(key.clone()).or_insert(id);
        if *entry != id {
            return *entry;
        }

        self.by_id.insert(id, Profile { id, key });
        id
    }

    /// Get a profile by id.
    pub fn get(&self, id: ProfileId) -> Option<Profile> {
        self.by_id.get(&id).map(|entry| entry.clone())
    }
}
