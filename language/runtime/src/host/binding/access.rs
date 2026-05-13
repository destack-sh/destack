use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::{BindingDescriptor, BindingEffect, BindingReplayPayload};
use crate::world::policy::ActionSet;
use destack_workspace::{ExecutionMode, ReplayPayloadMode, RuntimeOptions};

/// Per-worker binding access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingAccess {
    /// Effective execution mode for external calls.
    mode: ExecutionMode,
    /// Default replay payload when no world rule matches.
    default_replay_payload: BindingReplayPayload,
    /// Allowed actions when runtime security checks are enabled.
    allowed_actions: Option<ActionSet>,
}

impl BindingAccess {
    /// Create a binding access from an execution mode.
    pub fn new(mode: ExecutionMode) -> Self {
        Self {
            mode,
            default_replay_payload: BindingReplayPayload::Results,
            allowed_actions: None,
        }
    }

    /// Apply runtime defaults to this access.
    pub fn apply_runtime_defaults(&mut self, options: &RuntimeOptions) {
        self.apply_image_defaults(options.execution_mode(), options.trace.payload);
    }

    /// Apply captured access defaults without requiring full runtime options.
    pub fn apply_image_defaults(
        &mut self,
        execution: ExecutionMode,
        replay_payload: ReplayPayloadMode,
    ) {
        self.mode = execution;
        self.default_replay_payload = replay_payload.into();
    }

    /// Set the allowed action set used for requirement checks.
    pub fn set_allowed_actions(&mut self, actions: ActionSet) {
        self.allowed_actions = Some(actions);
    }

    /// Return the allowed action set when requirement checks are enabled.
    pub fn allowed_actions(&self) -> Option<&ActionSet> {
        self.allowed_actions.as_ref()
    }

    /// Return the default replay payload policy.
    pub fn default_replay_payload(&self) -> BindingReplayPayload {
        self.default_replay_payload
    }

    /// Validate one binding descriptor.
    #[inline]
    pub fn ensure_allowed(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        // action requirements
        if let Some(action) = self.missing_required_action(spec) {
            return Err(RuntimeError::ActionDenied {
                name: spec.name.to_string(),
                action: action.to_string(),
            }
            .boxed());
        }

        // execution mode
        if !self.allows_effect(spec.effect) {
            return Err(RuntimeError::PolicyViolation {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        Ok(())
    }

    /// Return the first missing required action for one binding.
    fn missing_required_action(&self, spec: BindingDescriptor) -> Option<&'static str> {
        match &self.allowed_actions {
            Some(actions) => spec.missing_requirement(actions),
            None => None,
        }
    }

    /// Return whether one binding effect can run under this access.
    const fn allows_effect(&self, effect: BindingEffect) -> bool {
        match self.mode {
            ExecutionMode::Fast => true,
            ExecutionMode::Deterministic => {
                matches!(effect, BindingEffect::Pure | BindingEffect::Deterministic)
            }
            ExecutionMode::Record | ExecutionMode::Replay => {
                !matches!(effect, BindingEffect::ExternalNonRecordable)
            }
        }
    }
}

impl From<ReplayPayloadMode> for BindingReplayPayload {
    /// Convert workspace replay payload mode to binding replay payload policy.
    fn from(mode: ReplayPayloadMode) -> Self {
        match mode {
            ReplayPayloadMode::ResultsOnly => Self::Results,
            ReplayPayloadMode::ArgumentsAndResults => Self::ArgumentsAndResults,
        }
    }
}
