use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::bindings::{
    BindingDescriptor, BindingEffectMask, BindingId, BindingReplayPayload, BindingScope,
};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::policy::{Effect, Policy, Rule, Selector, matches_selector};
use destack_workspace::{
    BindingEngine, ExecutionMode, ReplayPayloadMode, RuntimeAccess, RuntimeOptions, RuntimeWorld,
};
use rustc_hash::FxHashMap;

/// Access rule derived from runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AccessRule {
    /// Filter clause for this rule.
    when: Selector,
    /// Access action when the rule matches.
    access: RuntimeAccess,
}

/// World rule derived from runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WorldRule {
    /// Filter clause for this rule.
    when: Selector,
    /// World action when the rule matches.
    world: RuntimeWorld,
}

/// Replay payload rule derived from runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayPayloadRule {
    /// Filter clause for this rule.
    when: Selector,
    /// Replay payload policy when the rule matches.
    payload: BindingReplayPayload,
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
    default_replay_payload: BindingReplayPayload,
    /// Active capability set used for binding requirement checks.
    capabilities: PlatformCapabilitySet,
    /// Whether capability requirements are enforced.
    enforce_capability_requirements: bool,
    /// Ordered access rules with first match semantics.
    access_rules: Vec<AccessRule>,
    /// Ordered world rules with first match semantics.
    world_rules: Vec<WorldRule>,
    /// Ordered replay payload rules with first match semantics.
    replay_payload_rules: Vec<ReplayPayloadRule>,

    /// Compiled access decisions for any engine calls.
    access_compiled_any: FxHashMap<BindingId, RuntimeAccess>,
    /// Compiled access decisions for VM calls.
    access_compiled_vm: FxHashMap<BindingId, RuntimeAccess>,
    /// Compiled access decisions for native calls.
    access_compiled_native: FxHashMap<BindingId, RuntimeAccess>,
    /// Compiled world decisions for any engine calls.
    world_compiled_any: FxHashMap<BindingId, RuntimeWorld>,
    /// Compiled world decisions for VM calls.
    world_compiled_vm: FxHashMap<BindingId, RuntimeWorld>,
    /// Compiled world decisions for native calls.
    world_compiled_native: FxHashMap<BindingId, RuntimeWorld>,
    /// Compiled replay payload decisions for any engine calls.
    replay_payload_compiled_any: FxHashMap<BindingId, BindingReplayPayload>,
    /// Compiled replay payload decisions for VM calls.
    replay_payload_compiled_vm: FxHashMap<BindingId, BindingReplayPayload>,
    /// Compiled replay payload decisions for native calls.
    replay_payload_compiled_native: FxHashMap<BindingId, BindingReplayPayload>,
    /// Compiled capability requirement satisfaction by binding id.
    capability_requirements_compiled: FxHashMap<BindingId, bool>,
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
            default_replay_payload: BindingReplayPayload::Results,
            capabilities: PlatformCapabilitySet::new(),
            enforce_capability_requirements: false,
            access_rules: Vec::new(),
            world_rules: Vec::new(),
            replay_payload_rules: Vec::new(),
            access_compiled_any: FxHashMap::default(),
            access_compiled_vm: FxHashMap::default(),
            access_compiled_native: FxHashMap::default(),
            world_compiled_any: FxHashMap::default(),
            world_compiled_vm: FxHashMap::default(),
            world_compiled_native: FxHashMap::default(),
            replay_payload_compiled_any: FxHashMap::default(),
            replay_payload_compiled_vm: FxHashMap::default(),
            replay_payload_compiled_native: FxHashMap::default(),
            capability_requirements_compiled: FxHashMap::default(),
        }
    }

    /// Apply runtime defaults to this policy without loading control rules.
    pub fn apply_runtime_defaults(&mut self, options: &RuntimeOptions) {
        // align execution mode derived behavior
        self.mode = options.execution;
        self.allowed = allowed_effects_for_mode(self.mode);

        // apply default access policy
        self.default_access = options.access;
        self.default_world = options.world;
        self.default_replay_payload = replay_payload_from_mode(options.replay.payload);

        // defaults can change compiled outcomes for unmatched descriptors
        self.clear_compiled();
    }

    /// Apply runtime options to this policy.
    pub fn apply_runtime_options(&mut self, options: &RuntimeOptions) {
        self.apply_runtime_defaults(options);
        let policy = Policy::from_workspace_policy_rules(&options.rules);
        self.apply_policy(&policy);
    }

    /// Apply one runtime control set to this policy.
    pub fn apply_policy(&mut self, control: &Policy) {
        // materialize startup rules from runtime actions
        let rules = control.startup_rules();

        // rebuild ordered access rules
        self.access_rules.clear();
        self.access_rules
            .extend(rules.iter().filter_map(rule_to_access_rule));

        // rebuild ordered world rules
        self.world_rules.clear();
        self.world_rules
            .extend(rules.iter().filter_map(rule_to_world_rule));

        // rebuild ordered replay payload rules
        self.replay_payload_rules.clear();
        self.replay_payload_rules
            .extend(rules.iter().filter_map(rule_to_replay_payload_rule));

        // clear compiled lookups and rebuild from caller supplied descriptors
        self.clear_compiled();
    }

    /// Set the active capability set used for requirement checks.
    pub fn set_capabilities(&mut self, capabilities: PlatformCapabilitySet) {
        // replace active capabilities
        self.capabilities = capabilities;

        // enforce requirements only when explicit capabilities are configured
        self.enforce_capability_requirements = !self.capabilities.is_empty();

        // capability requirement decisions depend on this state
        self.clear_compiled();
    }

    /// Return the active capability set.
    pub fn capabilities(&self) -> &PlatformCapabilitySet {
        &self.capabilities
    }

    /// Set whether capability requirements are enforced.
    pub fn set_capability_requirements_enforced(&mut self, is_enforced: bool) {
        // update capability requirement enforcement mode
        self.enforce_capability_requirements = is_enforced;

        // capability requirement decisions depend on this state
        self.clear_compiled();
    }

    /// Return whether capability requirements are currently enforced.
    pub fn is_capability_requirements_enforced(&self) -> bool {
        self.enforce_capability_requirements
    }

    /// Compile policy decisions for one binding descriptor.
    pub fn compile_descriptor(&mut self, spec: BindingDescriptor) {
        // compile capability requirement satisfaction for this binding
        let has_capabilities = self.resolve_capability_requirements_uncached(spec);
        self.capability_requirements_compiled
            .insert(spec.id, has_capabilities);

        // compile access decisions for all engine variants
        let access_any = self.resolve_access_uncached(spec, None);
        let access_vm = self.resolve_access_uncached(spec, Some(BindingEngine::Vm));
        let access_native = self.resolve_access_uncached(spec, Some(BindingEngine::Native));

        // store access decisions in the compiled caches
        self.access_compiled_any.insert(spec.id, access_any);
        self.access_compiled_vm.insert(spec.id, access_vm);
        self.access_compiled_native.insert(spec.id, access_native);

        // compile world decisions for all engine variants
        let world_any = self.resolve_world_uncached(spec, None);
        let world_vm = self.resolve_world_uncached(spec, Some(BindingEngine::Vm));
        let world_native = self.resolve_world_uncached(spec, Some(BindingEngine::Native));

        // store world decisions in the compiled caches
        self.world_compiled_any.insert(spec.id, world_any);
        self.world_compiled_vm.insert(spec.id, world_vm);
        self.world_compiled_native.insert(spec.id, world_native);

        // compile replay payload decisions for all engine variants
        let replay_payload_any = self.resolve_replay_payload_uncached(spec, None);
        let replay_payload_vm = self.resolve_replay_payload_uncached(spec, Some(BindingEngine::Vm));
        let replay_payload_native =
            self.resolve_replay_payload_uncached(spec, Some(BindingEngine::Native));

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

    /// Validate one binding descriptor for one engine.
    #[inline]
    pub fn check_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeResult<()> {
        // reject capability requirement mismatches first
        if !self.resolve_capability_requirements(spec) {
            return Err(RuntimeError::PolicyViolation {
                name: spec.name.to_string(),
            }
            .boxed());
        }

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

        Ok(())
    }

    /// Validate a binding descriptor and resolve world for one engine.
    #[inline]
    pub fn check_and_resolve_world_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeResult<RuntimeWorld> {
        // run policy checks before world routing
        self.check_for_engine(spec, engine)?;

        Ok(self.resolve_world(spec, engine))
    }

    /// Resolve the effective world for one binding and engine.
    #[inline]
    pub fn resolve_world_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeWorld {
        self.resolve_world(spec, engine)
    }

    /// Resolve the replay payload policy for one binding and engine.
    #[inline]
    pub fn resolve_replay_payload_for_engine(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> BindingReplayPayload {
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
        self.capability_requirements_compiled.clear();
    }

    /// Resolve capability requirements for one binding.
    fn resolve_capability_requirements(&self, spec: BindingDescriptor) -> bool {
        // return compiled decisions when available
        if let Some(has_capabilities) = self.lookup_compiled_capability_requirements(spec.id) {
            return has_capabilities;
        }

        // fall back to direct requirement checks when uncached
        self.resolve_capability_requirements_uncached(spec)
    }

    /// Resolve capability requirements by checking the active capability set.
    fn resolve_capability_requirements_uncached(&self, spec: BindingDescriptor) -> bool {
        // disabled enforcement accepts all binding capability requirements
        if !self.enforce_capability_requirements {
            return true;
        }

        spec.requirements_satisfied_by(&self.capabilities)
    }

    /// Resolve the effective access policy for one binding and engine.
    fn resolve_access(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeAccess {
        // return compiled decisions when available
        if let Some(access) = self.lookup_compiled_access(spec.id, engine) {
            return access;
        }

        // fall back to rule walk resolution when uncached
        self.resolve_access_uncached(spec, engine)
    }

    /// Resolve the effective world policy for one binding and engine.
    fn resolve_world(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeWorld {
        // return compiled decisions when available
        if let Some(world) = self.lookup_compiled_world(spec.id, engine) {
            return world;
        }

        // fall back to rule walk resolution when uncached
        self.resolve_world_uncached(spec, engine)
    }

    /// Resolve the effective replay payload policy for one binding and engine.
    fn resolve_replay_payload(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> BindingReplayPayload {
        // return compiled decisions when available
        if let Some(payload) = self.lookup_compiled_replay_payload(spec.id, engine) {
            return payload;
        }

        // fall back to rule walk resolution when uncached
        self.resolve_replay_payload_uncached(spec, engine)
    }

    /// Resolve access policy by scanning ordered rules.
    fn resolve_access_uncached(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeAccess {
        // apply matching rules in declaration order
        for rule in &self.access_rules {
            if matches_selector(&rule.when, Some(spec), self.mode, engine, None) {
                return rule.access;
            }
        }

        // use default policy when no rule matches
        self.default_access
    }

    /// Resolve world policy by scanning ordered rules.
    fn resolve_world_uncached(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeWorld {
        // runtime scope bindings are always runtime owned and do not world route
        if spec.scope == BindingScope::Runtime {
            return RuntimeWorld::Host;
        }

        // apply matching world rules in declaration order
        for rule in &self.world_rules {
            if matches_selector(&rule.when, Some(spec), self.mode, engine, None) {
                return rule.world;
            }
        }

        // use default world when no rule matches
        self.default_world
    }

    /// Resolve replay payload policy by scanning ordered rules.
    fn resolve_replay_payload_uncached(
        &self,
        spec: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> BindingReplayPayload {
        // apply matching replay payload rules in declaration order
        for rule in &self.replay_payload_rules {
            if matches_selector(&rule.when, Some(spec), self.mode, engine, None) {
                return rule.payload;
            }
        }

        // use default replay payload when no rule matches
        self.default_replay_payload
    }

    /// Look up one compiled access decision.
    fn lookup_compiled_access(
        &self,
        id: BindingId,
        engine: Option<BindingEngine>,
    ) -> Option<RuntimeAccess> {
        // select cache by engine
        let cache = match engine {
            Some(BindingEngine::Vm) => &self.access_compiled_vm,
            Some(BindingEngine::Native) => &self.access_compiled_native,
            None => &self.access_compiled_any,
        };

        // return cached decision when available
        cache.get(&id).copied()
    }

    /// Look up one compiled world decision.
    fn lookup_compiled_world(
        &self,
        id: BindingId,
        engine: Option<BindingEngine>,
    ) -> Option<RuntimeWorld> {
        // select cache by engine
        let cache = match engine {
            Some(BindingEngine::Vm) => &self.world_compiled_vm,
            Some(BindingEngine::Native) => &self.world_compiled_native,
            None => &self.world_compiled_any,
        };

        // return cached decision when available
        cache.get(&id).copied()
    }

    /// Look up one compiled replay payload decision.
    fn lookup_compiled_replay_payload(
        &self,
        id: BindingId,
        engine: Option<BindingEngine>,
    ) -> Option<BindingReplayPayload> {
        // select cache by engine
        let cache = match engine {
            Some(BindingEngine::Vm) => &self.replay_payload_compiled_vm,
            Some(BindingEngine::Native) => &self.replay_payload_compiled_native,
            None => &self.replay_payload_compiled_any,
        };

        // return cached decision when available
        cache.get(&id).copied()
    }

    /// Look up one compiled capability requirement decision.
    fn lookup_compiled_capability_requirements(&self, id: BindingId) -> Option<bool> {
        self.capability_requirements_compiled.get(&id).copied()
    }
}

impl Default for BindingPolicy {
    /// Create the default policy for fast execution mode.
    fn default() -> Self {
        Self::new(ExecutionMode::Fast)
    }
}

/// Convert one rule into an access rule when the action sets access.
fn rule_to_access_rule(rule: &Rule) -> Option<AccessRule> {
    // keep only access action rules for policy checks
    let Effect::SetAccess { access } = &rule.action else {
        return None;
    };

    Some(AccessRule {
        when: rule.when.clone(),
        access: *access,
    })
}

/// Convert one rule into a world rule when the action sets world.
fn rule_to_world_rule(rule: &Rule) -> Option<WorldRule> {
    // keep only world action rules for world routing
    let Effect::SetWorld { world } = &rule.action else {
        return None;
    };

    Some(WorldRule {
        when: rule.when.clone(),
        world: *world,
    })
}

/// Convert one rule into a replay payload rule when the action sets replay payload.
fn rule_to_replay_payload_rule(rule: &Rule) -> Option<ReplayPayloadRule> {
    // keep only replay payload policy rules
    let Effect::SetReplay { payload } = &rule.action else {
        return None;
    };

    Some(ReplayPayloadRule {
        when: rule.when.clone(),
        payload: replay_payload_from_mode(*payload),
    })
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
const fn replay_payload_from_mode(mode: ReplayPayloadMode) -> BindingReplayPayload {
    match mode {
        ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
        ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor_with_requirements() -> BindingDescriptor {
        BindingDescriptor::deterministic_with_requires(
            "destack.test.capability",
            "export function capability(): void",
            &["fs.read", "net.connect"],
        )
    }

    #[test]
    fn test_check_for_engine_allows_when_capability_enforcement_is_disabled() {
        // build one policy and descriptor with explicit requirements
        let policy = BindingPolicy::new(ExecutionMode::Fast);
        let descriptor = descriptor_with_requirements();

        // capability checks are skipped when enforcement is disabled
        let result = policy.check_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_for_engine_rejects_when_capability_requirements_are_missing() {
        // build one policy with strict capability requirement enforcement
        let mut policy = BindingPolicy::new(ExecutionMode::Fast);
        policy.set_capability_requirements_enforced(true);

        let descriptor = descriptor_with_requirements();
        policy.compile_descriptor(descriptor);

        // missing required capabilities should be rejected
        let result = policy.check_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(result.is_err());
    }

    #[test]
    fn test_check_for_engine_accepts_when_capability_requirements_are_satisfied() {
        // build one policy and set required capabilities before compiling decisions
        let mut policy = BindingPolicy::new(ExecutionMode::Fast);
        policy.set_capability_requirements_enforced(true);
        policy.set_capabilities(PlatformCapabilitySet::from_names([
            "fs.read",
            "net.connect",
        ]));

        let descriptor = descriptor_with_requirements();
        policy.compile_descriptor(descriptor);

        // all required capabilities should allow the call
        let result = policy.check_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_for_engine_recomputes_when_capabilities_change() {
        // compile one descriptor with all required capabilities present
        let mut policy = BindingPolicy::new(ExecutionMode::Fast);
        policy.set_capabilities(PlatformCapabilitySet::from_names([
            "fs.read",
            "net.connect",
        ]));

        let descriptor = descriptor_with_requirements();
        policy.compile_descriptor(descriptor);

        // first pass should be accepted
        let first_result = policy.check_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(first_result.is_ok());

        // drop one required capability and recompile descriptor decisions
        policy.set_capabilities(PlatformCapabilitySet::from_names(["fs.read"]));
        policy.compile_descriptor(descriptor);

        // second pass should be rejected after capability update
        let second_result = policy.check_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(second_result.is_err());
    }
}
