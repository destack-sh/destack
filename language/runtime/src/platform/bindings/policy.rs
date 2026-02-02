use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::{BindingDescriptor, BindingEffectMask};

/// Determinism policy for runtime scheduling and I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeterminismPolicy {
    /// Best-effort execution without determinism guarantees.
    #[default]
    BestEffort,
    /// Deterministic scheduling with controlled randomness.
    Deterministic,
}

/// Replay policy for external effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplayMode {
    /// Disable record/replay.
    #[default]
    Off,
    /// Record external effects for replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

/// Policy configuration for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingPolicy {
    /// Determinism policy for external calls.
    determinism: DeterminismPolicy,
    /// Replay policy for external effects.
    replay: ReplayMode,
    /// Allowed binding effects for this policy.
    allowed: BindingEffectMask,
}

impl BindingPolicy {
    /// Create a binding policy from determinism and replay settings.
    pub const fn new(determinism: DeterminismPolicy, replay: ReplayMode) -> Self {
        let allowed = allowed_effects_for_policy(determinism, replay);
        Self {
            determinism,
            replay,
            allowed,
        }
    }

    /// Return the determinism policy.
    pub const fn determinism(self) -> DeterminismPolicy {
        self.determinism
    }

    /// Return the replay mode.
    pub const fn replay(self) -> ReplayMode {
        self.replay
    }

    /// Validate a binding descriptor against policy.
    #[inline]
    pub fn check(self, spec: BindingDescriptor) -> RuntimeResult<()> {
        if !self.allowed.allows(spec.effect_mask) {
            return Err(RuntimeError::policy_violation(spec.name.to_string()).boxed());
        }

        Ok(())
    }
}

impl Default for BindingPolicy {
    fn default() -> Self {
        Self::new(DeterminismPolicy::BestEffort, ReplayMode::Off)
    }
}

/// Calculate the allowed effects for a policy.
const fn allowed_effects_for_policy(
    determinism: DeterminismPolicy,
    replay: ReplayMode,
) -> BindingEffectMask {
    let mut mask = BindingEffectMask::PURE;
    mask.0 |= BindingEffectMask::DETERMINISTIC.0;

    let allow_external = match replay {
        ReplayMode::Off => matches!(determinism, DeterminismPolicy::BestEffort),
        ReplayMode::Record => true,
        ReplayMode::Replay => false,
    };

    if allow_external {
        match replay {
            ReplayMode::Record => {
                mask.0 |= BindingEffectMask::EXTERNAL_RECORDABLE.0;
            }
            ReplayMode::Off => {
                mask.0 |= BindingEffectMask::EXTERNAL_RECORDABLE.0;
                mask.0 |= BindingEffectMask::EXTERNAL_NONRECORDABLE.0;
            }
            ReplayMode::Replay => {}
        }
    }

    mask
}
