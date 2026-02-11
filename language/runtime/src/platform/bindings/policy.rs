use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::{
    BindingBlocking, BindingDescriptor, BindingEffectMask, BindingScope, EffectClass, ReplayPolicy,
};
use destack_source::matches as glob_matches;
use destack_workspace::{
    RuntimeAccess, RuntimeEffect, RuntimeFilter, RuntimeFilterBlocking, RuntimeFilterEffect,
    RuntimeFilterEngine, RuntimeFilterScope, RuntimeOptions, RuntimePolicyEffect, RuntimeRule,
};

/// Execution mode for the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ExecutionMode {
    /// Fast execution without determinism guarantees.
    #[default]
    Fast,
    /// Deterministic scheduling with controlled randomness.
    Deterministic,
    /// Record external effects for deterministic replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

/// Engine kind for one binding call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEngine {
    /// VM engine call.
    Vm,
    /// Native engine call.
    Native,
}

/// Access rule derived from runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AccessRule {
    /// Filter clause for this rule.
    when: RuntimeFilter,
    /// Access action when the rule matches.
    access: RuntimeAccess,
}

/// Policy configuration for external bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingPolicy {
    /// Execution mode for external calls.
    mode: ExecutionMode,
    /// Allowed binding effects for this policy.
    allowed: BindingEffectMask,
    /// Default access action for unmatched bindings.
    default_access: RuntimeAccess,
    /// Ordered access rules with last-match-wins semantics.
    access_rules: Vec<AccessRule>,
}

impl BindingPolicy {
    /// Create a binding policy from an execution mode.
    pub const fn new(mode: ExecutionMode) -> Self {
        let allowed = allowed_effects_for_mode(mode);
        Self {
            mode,
            allowed,
            default_access: RuntimeAccess::Allow,
            access_rules: Vec::new(),
        }
    }

    /// Apply runtime options to this policy.
    pub fn apply_runtime_options(&mut self, options: &RuntimeOptions) {
        // align execution mode derived behavior
        self.mode = options.execution.into();
        self.allowed = allowed_effects_for_mode(self.mode);

        // apply default access policy
        self.default_access = options.access;

        // rebuild ordered access rules
        self.access_rules.clear();
        self.access_rules
            .extend(options.rules.iter().filter_map(rule_to_access_rule));
    }

    /// Return the execution mode.
    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    /// Validate a binding descriptor against policy.
    #[inline]
    pub fn check(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        self.check_for_engine(spec, None)
    }

    /// Validate a binding descriptor against policy for one engine.
    #[inline]
    pub fn check_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeResult<()> {
        // reject disallowed effect classes first
        if !self.allowed.allows(spec.effect_mask) {
            return Err(RuntimeError::PolicyViolation {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        // resolve access with last-match-wins semantics
        let access = self.resolve_access(spec, engine);
        if access == RuntimeAccess::Deny {
            return Err(RuntimeError::PolicyViolation {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        Ok(())
    }

    fn resolve_access(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeAccess {
        // seed from the default policy
        let mut access = self.default_access;

        // apply matching rules in declaration order
        for rule in &self.access_rules {
            if matches_rule(&rule.when, spec, self.mode, engine) {
                access = rule.access;
            }
        }

        access
    }
}

impl Default for BindingPolicy {
    fn default() -> Self {
        Self::new(ExecutionMode::Fast)
    }
}

fn rule_to_access_rule(rule: &RuntimeRule) -> Option<AccessRule> {
    // keep only access action rules for policy checks
    let RuntimeEffect::Policy {
        policy: RuntimePolicyEffect::SetAccess { access },
    } = &rule.effect
    else {
        return None;
    };

    Some(AccessRule {
        when: rule.when.clone(),
        access: *access,
    })
}

fn matches_rule(
    filter: &RuntimeFilter,
    spec: BindingDescriptor,
    mode: ExecutionMode,
    engine: Option<PolicyEngine>,
) -> bool {
    // match binding name glob
    if let Some(pattern) = &filter.binding
        && !glob_match(pattern, spec.name)
    {
        return false;
    }

    // match module name glob
    if let Some(pattern) = &filter.module {
        let module_name = spec.name.strip_prefix("destack.").unwrap_or(spec.name);
        if !glob_match(pattern, module_name) {
            return false;
        }
    }

    // match component name glob
    if let Some(pattern) = &filter.component {
        let component_name = binding_component_name(spec.name);
        if !glob_match(pattern, component_name) {
            return false;
        }
    }

    // match any required capability
    if let Some(pattern) = &filter.capability {
        let has_match = spec
            .requires()
            .iter()
            .any(|capability| glob_match(pattern, capability));
        if !has_match {
            return false;
        }
    }

    // match engine selector when present
    if let Some(rule_engine) = filter.engine {
        let Some(engine) = engine else {
            return false;
        };
        if !matches_engine(rule_engine, engine) {
            return false;
        }
    }

    // match execution mode selector when present
    if let Some(modes) = &filter.execution_modes {
        let matches_mode = modes
            .iter()
            .copied()
            .any(|rule_mode| rule_mode == mode.into());
        if !matches_mode {
            return false;
        }
    }

    // match platform selector when present
    if let Some(platforms) = &filter.platforms {
        let current_platform = runtime_platform_name();
        let matches_platform = platforms
            .iter()
            .any(|pattern| glob_match(pattern, current_platform));
        if !matches_platform {
            return false;
        }
    }

    // match binding scope selector when present
    if let Some(scope) = filter.scope
        && !matches_scope(scope, spec.scope())
    {
        return false;
    }

    // match blocking selector when present
    if let Some(blocking) = filter.blocking
        && !matches_blocking(blocking, spec.blocking())
    {
        return false;
    }

    // match effect selector when present
    if let Some(effect) = filter.effect
        && !matches_effect(effect, spec.effect_class)
    {
        return false;
    }

    true
}

fn matches_engine(rule_engine: RuntimeFilterEngine, engine: PolicyEngine) -> bool {
    matches!(
        (rule_engine, engine),
        (RuntimeFilterEngine::Vm, PolicyEngine::Vm)
            | (RuntimeFilterEngine::Native, PolicyEngine::Native)
    )
}

fn matches_scope(rule_scope: RuntimeFilterScope, scope: BindingScope) -> bool {
    matches!(
        (rule_scope, scope),
        (RuntimeFilterScope::Os, BindingScope::Os)
            | (RuntimeFilterScope::Runtime, BindingScope::Runtime)
            | (RuntimeFilterScope::Hybrid, BindingScope::Hybrid)
    )
}

fn matches_blocking(rule_blocking: RuntimeFilterBlocking, blocking: BindingBlocking) -> bool {
    matches!(
        (rule_blocking, blocking),
        (RuntimeFilterBlocking::Always, BindingBlocking::Always)
            | (RuntimeFilterBlocking::Never, BindingBlocking::Never)
            | (RuntimeFilterBlocking::Sometimes, BindingBlocking::Sometimes)
    )
}

fn matches_effect(rule_effect: RuntimeFilterEffect, effect_class: EffectClass) -> bool {
    matches!(
        (rule_effect, effect_class),
        (RuntimeFilterEffect::Pure, EffectClass::Pure)
            | (
                RuntimeFilterEffect::Deterministic,
                EffectClass::Deterministic
            )
            | (
                RuntimeFilterEffect::ExternalRecordable,
                EffectClass::External {
                    replay: ReplayPolicy::Recordable,
                },
            )
            | (
                RuntimeFilterEffect::ExternalNonRecordable,
                EffectClass::External {
                    replay: ReplayPolicy::NonRecordable,
                },
            )
    )
}

fn glob_match(pattern: &str, text: &str) -> bool {
    glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
}

fn binding_component_name(binding_name: &str) -> &str {
    let stripped_name = binding_name
        .strip_prefix("destack.")
        .unwrap_or(binding_name);
    let mut component_segments = stripped_name.splitn(2, '.');
    component_segments.next().unwrap_or("unknown")
}

fn runtime_platform_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        return "linux";
    }

    #[cfg(target_os = "macos")]
    {
        "macos"
    }

    #[cfg(target_os = "ios")]
    {
        return "ios";
    }

    #[cfg(target_os = "windows")]
    {
        return "windows";
    }

    #[cfg(target_os = "wasi")]
    {
        return "wasi";
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "wasi"
    )))]
    {
        return "unknown";
    }
}

impl From<destack_workspace::ExecutionMode> for ExecutionMode {
    fn from(mode: destack_workspace::ExecutionMode) -> Self {
        match mode {
            destack_workspace::ExecutionMode::Fast => ExecutionMode::Fast,
            destack_workspace::ExecutionMode::Deterministic => ExecutionMode::Deterministic,
            destack_workspace::ExecutionMode::Record => ExecutionMode::Record,
            destack_workspace::ExecutionMode::Replay => ExecutionMode::Replay,
        }
    }
}

impl From<ExecutionMode> for destack_workspace::ExecutionMode {
    fn from(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Fast => destack_workspace::ExecutionMode::Fast,
            ExecutionMode::Deterministic => destack_workspace::ExecutionMode::Deterministic,
            ExecutionMode::Record => destack_workspace::ExecutionMode::Record,
            ExecutionMode::Replay => destack_workspace::ExecutionMode::Replay,
        }
    }
}

/// Calculate the allowed effects for a mode.
const fn allowed_effects_for_mode(mode: ExecutionMode) -> BindingEffectMask {
    let mut mask = BindingEffectMask::PURE;
    mask.0 |= BindingEffectMask::DETERMINISTIC.0;

    match mode {
        ExecutionMode::Fast => {
            mask.0 |= BindingEffectMask::EXTERNAL_RECORDABLE.0;
            mask.0 |= BindingEffectMask::EXTERNAL_NONRECORDABLE.0;
        }
        ExecutionMode::Deterministic => {}
        ExecutionMode::Record | ExecutionMode::Replay => {
            mask.0 |= BindingEffectMask::EXTERNAL_RECORDABLE.0;
        }
    }

    mask
}
