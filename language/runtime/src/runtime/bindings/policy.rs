use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::bindings::{BindingDescriptor, BindingEffectMask, BindingReplayPayload};
use crate::runtime::capability::PlatformCapabilitySet;
use destack_workspace::{
    BindingEngine, ExecutionMode, ReplayPayloadMode, RuntimeAccess, RuntimeOptions, RuntimeWorld,
};

/// Policy configuration for external bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingPolicy {
    /// Execution mode for external calls.
    mode: ExecutionMode,
    /// Allowed binding effects for this policy.
    allowed: BindingEffectMask,
    /// Default access action when no world rule matches.
    default_access: RuntimeAccess,
    /// Default world action when no world rule matches.
    default_world: RuntimeWorld,
    /// Default replay payload when no world rule matches.
    default_replay_payload: BindingReplayPayload,
    /// Active capability set used for binding requirement checks.
    capabilities: PlatformCapabilitySet,
    /// Whether capability requirements are enforced.
    enforce_capability_requirements: bool,
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
        }
    }

    /// Apply runtime defaults to this policy.
    pub fn apply_runtime_defaults(&mut self, options: &RuntimeOptions) {
        self.apply_image_defaults(
            options.execution,
            options.access,
            options.world,
            options.replay.payload,
        );
    }

    /// Apply captured policy defaults without requiring full runtime options.
    pub fn apply_image_defaults(
        &mut self,
        execution: ExecutionMode,
        access: RuntimeAccess,
        world: RuntimeWorld,
        replay_payload: ReplayPayloadMode,
    ) {
        // align execution mode derived behavior
        self.mode = execution;
        self.allowed = allowed_effects_for_mode(self.mode);

        // apply default dispatch behavior
        self.default_access = access;
        self.default_world = world;
        self.default_replay_payload = replay_payload_from_mode(replay_payload);
    }

    /// Set the active capability set used for requirement checks.
    pub fn set_capabilities(&mut self, capabilities: PlatformCapabilitySet) {
        // replace active capabilities
        self.capabilities = capabilities;

        // enforce requirements only when explicit capabilities are configured
        self.enforce_capability_requirements = !self.capabilities.is_empty();
    }

    /// Return the active capability set.
    pub fn capabilities(&self) -> &PlatformCapabilitySet {
        &self.capabilities
    }

    /// Set whether capability requirements are enforced.
    pub fn set_capability_requirements_enforced(&mut self, is_enforced: bool) {
        self.enforce_capability_requirements = is_enforced;
    }

    /// Return whether capability requirements are currently enforced.
    pub fn is_capability_requirements_enforced(&self) -> bool {
        self.enforce_capability_requirements
    }

    /// Return the execution mode.
    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    /// Return the default access action.
    pub fn default_access(&self) -> RuntimeAccess {
        self.default_access
    }

    /// Return the default world action.
    pub fn default_world(&self) -> RuntimeWorld {
        self.default_world
    }

    /// Return the default replay payload action.
    pub fn default_replay_payload(&self) -> BindingReplayPayload {
        self.default_replay_payload
    }

    /// Validate one binding descriptor for one engine.
    #[inline]
    pub fn ensure_allowed_for_engine(
        &self,
        spec: BindingDescriptor,
        _engine: Option<BindingEngine>,
    ) -> RuntimeResult<()> {
        // reject capability requirement mismatches first
        if !self.capability_requirements_satisfied(spec) {
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

        Ok(())
    }

    /// Return whether capability requirements are satisfied for one binding.
    fn capability_requirements_satisfied(&self, spec: BindingDescriptor) -> bool {
        if !self.enforce_capability_requirements {
            return true;
        }

        spec.requirements_satisfied_by(&self.capabilities)
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
    fn test_ensure_allowed_for_engine_allows_when_capability_enforcement_is_disabled() {
        // build one policy and descriptor with explicit requirements
        let policy = BindingPolicy::new(ExecutionMode::Fast);
        let descriptor = descriptor_with_requirements();

        // capability checks are skipped when enforcement is disabled
        let result = policy.ensure_allowed_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_allowed_for_engine_rejects_when_capability_requirements_are_missing() {
        // build one policy with strict capability requirement enforcement
        let mut policy = BindingPolicy::new(ExecutionMode::Fast);
        policy.set_capability_requirements_enforced(true);

        let descriptor = descriptor_with_requirements();

        // missing required capabilities should be rejected
        let result = policy.ensure_allowed_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_allowed_for_engine_accepts_when_capability_requirements_are_satisfied() {
        // build one policy and set required capabilities before checks
        let mut policy = BindingPolicy::new(ExecutionMode::Fast);
        policy.set_capability_requirements_enforced(true);
        policy.set_capabilities(PlatformCapabilitySet::from_names([
            "fs.read",
            "net.connect",
        ]));

        let descriptor = descriptor_with_requirements();

        // all required capabilities should allow the call
        let result = policy.ensure_allowed_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_allowed_for_engine_recomputes_when_capabilities_change() {
        // start with one policy where all requirements are satisfied
        let mut policy = BindingPolicy::new(ExecutionMode::Fast);
        policy.set_capabilities(PlatformCapabilitySet::from_names([
            "fs.read",
            "net.connect",
        ]));

        let descriptor = descriptor_with_requirements();

        // first pass should be accepted
        let first_result =
            policy.ensure_allowed_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(first_result.is_ok());

        // drop one required capability and check again
        policy.set_capabilities(PlatformCapabilitySet::from_names(["fs.read"]));

        // second pass should be rejected after capability update
        let second_result =
            policy.ensure_allowed_for_engine(descriptor, Some(BindingEngine::Native));
        assert!(second_result.is_err());
    }
}
