use tspp_program as program;
use tspp_repository::ExecutionMode;

use crate::binding::ReplayPayload;
use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Per-worker binding access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingAccess {
    /// Effective execution mode for external calls.
    mode: ExecutionMode,
    /// Default replay payload when no world rule matches.
    default_replay_payload: ReplayPayload,
}

impl BindingAccess {
    /// Create worker binding access.
    pub const fn new(mode: ExecutionMode, replay_payload: ReplayPayload) -> Self {
        Self {
            mode,
            default_replay_payload: replay_payload,
        }
    }

    /// Return the default replay payload policy.
    pub fn default_replay_payload(&self) -> ReplayPayload {
        self.default_replay_payload
    }

    /// Validate one binding declaration.
    #[inline]
    pub fn ensure_allowed(&self, binding: &program::Binding, name: &str) -> RuntimeResult<()> {
        if !self.allows(binding) {
            return Err(RuntimeError::policy_violation(name.to_string()).boxed());
        }

        Ok(())
    }

    /// Return whether one binding can run under this execution mode.
    const fn allows(&self, binding: &program::Binding) -> bool {
        match self.mode {
            ExecutionMode::Fast => true,
            ExecutionMode::Strict => !matches!(binding.effect, program::BindingEffect::External),
            ExecutionMode::Record | ExecutionMode::Replay => {
                !matches!(binding.replay, program::BindingReplay::Forbidden)
            }
        }
    }
}
