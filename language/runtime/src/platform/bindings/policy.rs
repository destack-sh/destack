use std::collections::HashMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::{
    BindingDescriptor, BindingEffectMask, BindingId, BindingScope, ReplayPayload,
};
use crate::runtime::rules::matches_runtime_filter;
use destack_workspace::{
    ReplayPayloadMode, RuntimeAccess, RuntimeEffect, RuntimeFilter, RuntimeOptions,
    RuntimePolicyEffect, RuntimeRule, RuntimeWorld,
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

/// World rule derived from runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WorldRule {
    /// Filter clause for this rule.
    when: RuntimeFilter,
    /// World action when the rule matches.
    world: RuntimeWorld,
}

/// Replay payload rule derived from runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayPayloadRule {
    /// Filter clause for this rule.
    when: RuntimeFilter,
    /// Replay payload policy when the rule matches.
    payload: ReplayPayload,
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
    /// Default world for unmatched bindings.
    default_world: RuntimeWorld,
    /// Default replay payload for unmatched bindings.
    default_replay_payload: ReplayPayload,
    /// Ordered access rules with first-match-wins semantics.
    access_rules: Vec<AccessRule>,
    /// Ordered world rules with first-match-wins semantics.
    world_rules: Vec<WorldRule>,
    /// Ordered replay payload rules with first-match-wins semantics.
    replay_payload_rules: Vec<ReplayPayloadRule>,
    /// Compiled access decisions for any-engine calls.
    access_compiled_any: HashMap<BindingId, RuntimeAccess>,
    /// Compiled access decisions for VM calls.
    access_compiled_vm: HashMap<BindingId, RuntimeAccess>,
    /// Compiled access decisions for native calls.
    access_compiled_native: HashMap<BindingId, RuntimeAccess>,
    /// Compiled world decisions for any-engine calls.
    world_compiled_any: HashMap<BindingId, RuntimeWorld>,
    /// Compiled world decisions for VM calls.
    world_compiled_vm: HashMap<BindingId, RuntimeWorld>,
    /// Compiled world decisions for native calls.
    world_compiled_native: HashMap<BindingId, RuntimeWorld>,
    /// Compiled replay payload decisions for any-engine calls.
    replay_payload_compiled_any: HashMap<BindingId, ReplayPayload>,
    /// Compiled replay payload decisions for VM calls.
    replay_payload_compiled_vm: HashMap<BindingId, ReplayPayload>,
    /// Compiled replay payload decisions for native calls.
    replay_payload_compiled_native: HashMap<BindingId, ReplayPayload>,
}

impl BindingPolicy {
    /// Create a binding policy from an execution mode.
    pub fn new(mode: ExecutionMode) -> Self {
        let allowed = allowed_effects_for_mode(mode);
        Self {
            mode,
            allowed,
            default_access: RuntimeAccess::Allow,
            default_world: RuntimeWorld::Host,
            default_replay_payload: ReplayPayload::Results,
            access_rules: Vec::new(),
            world_rules: Vec::new(),
            replay_payload_rules: Vec::new(),
            access_compiled_any: HashMap::new(),
            access_compiled_vm: HashMap::new(),
            access_compiled_native: HashMap::new(),
            world_compiled_any: HashMap::new(),
            world_compiled_vm: HashMap::new(),
            world_compiled_native: HashMap::new(),
            replay_payload_compiled_any: HashMap::new(),
            replay_payload_compiled_vm: HashMap::new(),
            replay_payload_compiled_native: HashMap::new(),
        }
    }

    /// Apply runtime options to this policy.
    pub fn apply_runtime_options(&mut self, options: &RuntimeOptions) {
        // align execution mode derived behavior
        self.mode = options.execution.into();
        self.allowed = allowed_effects_for_mode(self.mode);

        // apply default access policy
        self.default_access = options.access;
        self.default_world = options.world;
        self.default_replay_payload = replay_payload_from_mode(options.replay_log.payload);

        // rebuild ordered access rules
        self.access_rules.clear();
        self.access_rules
            .extend(options.rules.iter().filter_map(rule_to_access_rule));

        // rebuild ordered world rules
        self.world_rules.clear();
        self.world_rules
            .extend(options.rules.iter().filter_map(rule_to_world_rule));

        // rebuild ordered replay payload rules
        self.replay_payload_rules.clear();
        self.replay_payload_rules
            .extend(options.rules.iter().filter_map(rule_to_replay_payload_rule));

        // clear compiled lookups and rebuild from caller supplied descriptors
        self.clear_compiled();
    }

    /// Compile policy decisions for one binding descriptor.
    pub fn compile_descriptor(&mut self, spec: BindingDescriptor) {
        // compile access decisions for all engine variants
        let access_any = self.resolve_access_uncached(spec, None);
        let access_vm = self.resolve_access_uncached(spec, Some(PolicyEngine::Vm));
        let access_native = self.resolve_access_uncached(spec, Some(PolicyEngine::Native));

        // store access decisions in the compiled caches
        self.access_compiled_any.insert(spec.id, access_any);
        self.access_compiled_vm.insert(spec.id, access_vm);
        self.access_compiled_native.insert(spec.id, access_native);

        // compile world decisions for all engine variants
        let world_any = self.resolve_world_uncached(spec, None);
        let world_vm = self.resolve_world_uncached(spec, Some(PolicyEngine::Vm));
        let world_native = self.resolve_world_uncached(spec, Some(PolicyEngine::Native));

        // store world decisions in the compiled caches
        self.world_compiled_any.insert(spec.id, world_any);
        self.world_compiled_vm.insert(spec.id, world_vm);
        self.world_compiled_native.insert(spec.id, world_native);

        // compile replay payload decisions for all engine variants
        let replay_payload_any = self.resolve_replay_payload_uncached(spec, None);
        let replay_payload_vm = self.resolve_replay_payload_uncached(spec, Some(PolicyEngine::Vm));
        let replay_payload_native =
            self.resolve_replay_payload_uncached(spec, Some(PolicyEngine::Native));

        // store replay payload decisions in the compiled caches
        self.replay_payload_compiled_any
            .insert(spec.id, replay_payload_any);
        self.replay_payload_compiled_vm
            .insert(spec.id, replay_payload_vm);
        self.replay_payload_compiled_native
            .insert(spec.id, replay_payload_native);
    }

    /// Compile policy decisions for multiple binding descriptors.
    pub fn compile_descriptors(&mut self, descriptors: &[BindingDescriptor]) {
        // reset caches before recompiling
        self.clear_compiled();

        // compile all known binding descriptors
        for descriptor in descriptors {
            self.compile_descriptor(*descriptor);
        }
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

    /// Validate a binding descriptor and resolve world selection.
    #[inline]
    pub fn check_and_resolve_world(&self, spec: BindingDescriptor) -> RuntimeResult<RuntimeWorld> {
        self.check_and_resolve_world_for_engine(spec, None)
    }

    /// Validate a binding descriptor against policy for one engine.
    #[inline]
    pub fn check_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeResult<()> {
        self.check_and_resolve_world_for_engine(spec, engine)?;
        Ok(())
    }

    /// Validate a binding descriptor and resolve world for one engine.
    #[inline]
    pub fn check_and_resolve_world_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeResult<RuntimeWorld> {
        // reject disallowed effect classes first
        if !self.allowed.allows(spec.effect_mask) {
            return Err(RuntimeError::PolicyViolation {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        // resolve access using the compiled cache when available
        let access = self.resolve_access(spec, engine);
        if access == RuntimeAccess::Deny {
            return Err(RuntimeError::PolicyViolation {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        Ok(self.resolve_world(spec, engine))
    }

    /// Resolve the effective world for one binding and engine.
    #[inline]
    pub fn resolve_world_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeWorld {
        self.resolve_world(spec, engine)
    }

    /// Resolve the replay payload policy for one binding and engine.
    #[inline]
    pub fn resolve_replay_payload_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> ReplayPayload {
        self.resolve_replay_payload(spec, engine)
    }

    /// Clear all compiled policy lookup tables.
    pub fn clear_compiled(&mut self) {
        self.access_compiled_any.clear();
        self.access_compiled_vm.clear();
        self.access_compiled_native.clear();
        self.world_compiled_any.clear();
        self.world_compiled_vm.clear();
        self.world_compiled_native.clear();
        self.replay_payload_compiled_any.clear();
        self.replay_payload_compiled_vm.clear();
        self.replay_payload_compiled_native.clear();
    }

    fn resolve_access(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeAccess {
        // return compiled decisions when available
        if let Some(access) = self.lookup_compiled_access(spec.id, engine) {
            return access;
        }

        // fall back to rule-walk resolution when uncached
        self.resolve_access_uncached(spec, engine)
    }

    fn resolve_world(&self, spec: BindingDescriptor, engine: Option<PolicyEngine>) -> RuntimeWorld {
        // return compiled decisions when available
        if let Some(world) = self.lookup_compiled_world(spec.id, engine) {
            return world;
        }

        // fall back to rule-walk resolution when uncached
        self.resolve_world_uncached(spec, engine)
    }

    fn resolve_replay_payload(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> ReplayPayload {
        // return compiled decisions when available
        if let Some(payload) = self.lookup_compiled_replay_payload(spec.id, engine) {
            return payload;
        }

        // fall back to rule-walk resolution when uncached
        self.resolve_replay_payload_uncached(spec, engine)
    }

    fn resolve_access_uncached(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeAccess {
        // apply matching rules in declaration order
        for rule in &self.access_rules {
            if matches_runtime_filter(&rule.when, spec, self.mode, engine) {
                return rule.access;
            }
        }

        // use default policy when no rule matches
        self.default_access
    }

    fn resolve_world_uncached(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeWorld {
        // runtime-scope bindings are always runtime-owned and do not world-route
        if spec.scope == BindingScope::Runtime {
            return RuntimeWorld::Host;
        }

        // apply matching world rules in declaration order
        for rule in &self.world_rules {
            if matches_runtime_filter(&rule.when, spec, self.mode, engine) {
                return rule.world;
            }
        }

        // use default world when no rule matches
        self.default_world
    }

    fn resolve_replay_payload_uncached(
        &self,
        spec: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> ReplayPayload {
        // apply matching replay payload rules in declaration order
        for rule in &self.replay_payload_rules {
            if matches_runtime_filter(&rule.when, spec, self.mode, engine) {
                return rule.payload;
            }
        }

        // use default replay payload when no rule matches
        self.default_replay_payload
    }

    fn lookup_compiled_access(
        &self,
        id: BindingId,
        engine: Option<PolicyEngine>,
    ) -> Option<RuntimeAccess> {
        // select cache by engine
        let cache = match engine {
            Some(PolicyEngine::Vm) => &self.access_compiled_vm,
            Some(PolicyEngine::Native) => &self.access_compiled_native,
            None => &self.access_compiled_any,
        };

        // return cached decision when available
        cache.get(&id).copied()
    }

    fn lookup_compiled_world(
        &self,
        id: BindingId,
        engine: Option<PolicyEngine>,
    ) -> Option<RuntimeWorld> {
        // select cache by engine
        let cache = match engine {
            Some(PolicyEngine::Vm) => &self.world_compiled_vm,
            Some(PolicyEngine::Native) => &self.world_compiled_native,
            None => &self.world_compiled_any,
        };

        // return cached decision when available
        cache.get(&id).copied()
    }

    fn lookup_compiled_replay_payload(
        &self,
        id: BindingId,
        engine: Option<PolicyEngine>,
    ) -> Option<ReplayPayload> {
        // select cache by engine
        let cache = match engine {
            Some(PolicyEngine::Vm) => &self.replay_payload_compiled_vm,
            Some(PolicyEngine::Native) => &self.replay_payload_compiled_native,
            None => &self.replay_payload_compiled_any,
        };

        // return cached decision when available
        cache.get(&id).copied()
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

fn rule_to_world_rule(rule: &RuntimeRule) -> Option<WorldRule> {
    // keep only world action rules for world routing
    let RuntimeEffect::Policy {
        policy: RuntimePolicyEffect::SetWorld { world },
    } = &rule.effect
    else {
        return None;
    };

    Some(WorldRule {
        when: rule.when.clone(),
        world: *world,
    })
}

fn rule_to_replay_payload_rule(rule: &RuntimeRule) -> Option<ReplayPayloadRule> {
    // keep only replay payload policy rules
    let RuntimeEffect::Policy {
        policy: RuntimePolicyEffect::SetReplay { payload },
    } = &rule.effect
    else {
        return None;
    };

    Some(ReplayPayloadRule {
        when: rule.when.clone(),
        payload: replay_payload_from_mode(*payload),
    })
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

/// Convert workspace replay payload mode to runtime replay payload policy.
const fn replay_payload_from_mode(mode: ReplayPayloadMode) -> ReplayPayload {
    match mode {
        ReplayPayloadMode::ResultsOnly => ReplayPayload::Results,
        ReplayPayloadMode::ArgumentsAndResults => ReplayPayload::ArgumentsAndResults,
    }
}
