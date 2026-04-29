use destack_artifact::{HostEnvironmentKey, ProfileFlags};
use serde::{Deserialize, Serialize};

use crate::HostEnvironment;
use crate::config::CompilerOptions;

/// The stable profile id.
pub use destack_source::ProfileId;

/// Resolved environment values for one profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEnvironment {
    /// The exposed environment entries.
    pub values: Vec<(String, String)>,
    /// The effective node environment mode.
    pub node_env: Option<String>,
    /// True in development builds.
    pub dev: bool,
    /// True in production builds.
    pub prod: bool,
    /// True in test builds.
    pub test: bool,
}

impl ProfileEnvironment {
    /// Derive the effective node environment and mode flags.
    pub fn mode_from_key(
        key: &HostEnvironmentKey,
        host: &HostEnvironment,
        debug: bool,
    ) -> (Option<String>, bool, bool, bool) {
        let has_node_env = key.keys().iter().any(|key| key == "NODE_ENV");
        let mut node_env = if has_node_env {
            host.get("NODE_ENV").map(ToOwned::to_owned)
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

    /// Build one profile environment from one host environment key.
    pub fn from_key(key: &HostEnvironmentKey, host: &HostEnvironment, debug: bool) -> Self {
        let mut values = key
            .keys()
            .iter()
            .filter_map(|key| host.get(key).map(|value| (key.clone(), value.to_string())))
            .collect::<Vec<_>>();

        let has_node_env = key.keys().iter().any(|key| key == "NODE_ENV");
        let (node_env, dev, prod, test) = Self::mode_from_key(key, host, debug);

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
