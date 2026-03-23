use std::sync::atomic::{AtomicU32, Ordering};

use destack_artifact::{EnvSnapshot, ProfileFlags, ProfileKey};
use serde::{Deserialize, Serialize};

use crate::CompilerOptions;
use dashmap::DashMap;

// Re-export ProfileId and ProfileVersion from destack_source
pub use destack_source::{ProfileId, ProfileVersion};

/// A registered profile.
#[derive(Debug, Clone)]
pub struct Profile {
    /// The profile id.
    pub id: ProfileId,
    /// The canonical key for the profile.
    pub key: ProfileKey,
    /// The profile version for incremental comptime state.
    pub version: ProfileVersion,
    /// Resolved environment values (for import.meta.env).
    pub env: ProfileEnv,
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

        // compute resolved env from the snapshot
        let env = ProfileEnv::from_snapshot(&key.env, key.debug);
        self.profile_by_id.insert(
            id,
            Profile {
                id,
                key,
                version: ProfileVersion::INITIAL,
                env,
            },
        );
        id
    }

    /// Get the profile id for one canonical key.
    pub fn id_for_key(&self, key: &ProfileKey) -> Option<ProfileId> {
        self.profile_by_key.get(key).map(|entry| *entry)
    }

    /// Get the profile id for one stable key hash.
    pub fn id_for_stable_hash(&self, stable_hash: u64) -> Option<ProfileId> {
        for entry in self.profile_by_id.iter() {
            if entry.value().key.stable_hash() == stable_hash {
                return Some(*entry.key());
            }
        }

        None
    }

    /// Get a profile by id.
    pub fn get(&self, id: ProfileId) -> Option<Profile> {
        self.profile_by_id.get(&id).map(|entry| entry.clone())
    }

    /// Get the current version for one profile id.
    pub fn version(&self, id: ProfileId) -> Option<ProfileVersion> {
        self.profile_by_id.get(&id).map(|entry| entry.version)
    }

    /// Bump the version for a profile id.
    ///
    /// # Panics
    /// Panics if the profile id is missing.
    pub fn bump_version(&self, id: ProfileId) -> ProfileVersion {
        // update the profile version
        let mut entry = self
            .profile_by_id
            .get_mut(&id)
            .unwrap_or_else(|| panic!("missing profile data for {id:?}"));
        let next_version = entry.version.next();
        entry.version = next_version;
        next_version
    }

    /// Get the number of registered profiles.
    pub fn len(&self) -> usize {
        self.profile_by_id.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.profile_by_id.is_empty()
    }
}

/// Derive profile identity flags from compiler options.
pub fn profile_flags_for_compiler_options(options: &CompilerOptions) -> ProfileFlags {
    ProfileFlags {
        no_any: !options.no_any.is_allow(),
        no_unknown: !options.no_unknown.is_allow(),
        no_imprecise_primitives: !options.no_imprecise_primitives.is_allow(),
        no_implicit_conversions: !options.no_implicit_conversions.is_allow(),
        no_unsafe_type_assertions: !options.no_unsafe_type_assertions.is_allow(),
        no_must_assertions: !options.no_must_assertions.is_allow(),
        no_definite_assignment_assertions: !options.no_definite_assignment_assertions.is_allow(),
        no_custom_type_guards: !options.no_custom_type_guards.is_allow(),
        no_unsound_variance: !options.no_unsound_variance.is_allow(),
        no_unsound_narrowing: !options.no_unsound_narrowing.is_allow(),
        deep_readonly: !options.deep_readonly.is_allow(),
        no_untrusted_declarations: !options.no_untrusted_declarations.is_allow(),
        no_redeclared_locals: !options.no_redeclared_locals.is_allow(),
        no_implicit_managed: !options.no_implicit_managed.is_allow(),
        no_managed: !options.no_managed.is_allow(),
        no_runtime: !options.no_runtime.is_allow(),
        no_referential_equality: !options.no_referential_equality.is_allow(),
        no_dynamic_evaluation: !options.no_dynamic_evaluation.is_allow(),
        no_global_this: !options.no_global_this.is_allow(),
        no_dynamic_import: !options.no_dynamic_import.is_allow(),
        no_internal_import: !options.no_internal_import.is_allow(),
        no_dynamic_shapes: !options.no_dynamic_shapes.is_allow(),
        no_computed_property_access: !options.no_computed_property_access.is_allow(),
        no_proxy: !options.no_proxy.is_allow(),
        no_implicit_dynamic_dispatch: !options.no_implicit_dynamic_dispatch.is_allow(),
        no_exceptions: !options.no_exceptions.is_allow(),
        strict_builtin_iterator_return: options.strict_builtin_iterator_return,
    }
}

/// Resolved environment values for a profile.
/// This contains the actual env values that are exposed to import.meta.env.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEnv {
    /// The environment entries exposed to user code.
    pub values: Vec<(String, String)>,
    /// Node environment mode.
    pub node_env: Option<String>,
    /// True in development builds.
    pub dev: bool,
    /// True in production builds.
    pub prod: bool,
    /// True in test builds.
    pub test: bool,
}

impl ProfileEnv {
    /// Derive the NODE_ENV value and mode flags from a snapshot.
    pub fn mode_from_snapshot(
        snapshot: &EnvSnapshot,
        debug: bool,
    ) -> (Option<String>, bool, bool, bool) {
        let has_node_env = snapshot.keys().iter().any(|key| key == "NODE_ENV");
        let mut node_env = if has_node_env {
            std::env::var("NODE_ENV").ok()
        } else {
            None
        };

        if node_env.is_none() {
            node_env = Some(if debug {
                "development".to_string()
            } else {
                "production".to_string()
            });
        }

        let (dev, prod, test) = match node_env.as_deref() {
            Some("production") => (false, true, false),
            Some("test") => (false, false, true),
            Some("development") => (true, false, false),
            Some(_) | None => (debug, !debug, false),
        };

        (node_env, dev, prod, test)
    }

    /// Build ProfileEnv from the snapshot and debug flag.
    pub fn from_snapshot(snapshot: &EnvSnapshot, debug: bool) -> Self {
        let mut values = snapshot
            .keys()
            .iter()
            .filter_map(|key| std::env::var(key).ok().map(|value| (key.clone(), value)))
            .collect::<Vec<_>>();

        let has_node_env = snapshot.keys().iter().any(|key| key == "NODE_ENV");
        let (node_env, dev, prod, test) = Self::mode_from_snapshot(snapshot, debug);

        if has_node_env
            && let Some(node_env_value) = node_env.clone()
            && !values.iter().any(|(key, _)| key == "NODE_ENV")
        {
            values.push(("NODE_ENV".to_string(), node_env_value));
        }

        values.sort_by(|left, right| left.0.cmp(&right.0));

        Self {
            values,
            node_env,
            dev,
            prod,
            test,
        }
    }
}
