use destack_vm::Error;

use super::{BindingDescriptor, EffectClass, ReplayPolicy};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BindingPolicy {
    /// Determinism policy for external calls.
    pub determinism: DeterminismPolicy,
    /// Replay policy for external effects.
    pub replay: ReplayMode,
}

impl BindingPolicy {
    /// Validate a binding descriptor against policy.
    pub fn check(self, spec: BindingDescriptor) -> Result<(), Error> {
        // determine whether external effects are allowed
        let allowed = match spec.effect_class {
            EffectClass::External { replay } => match self.replay {
                ReplayMode::Off => match self.determinism {
                    DeterminismPolicy::BestEffort => true,
                    DeterminismPolicy::Deterministic => false,
                },
                ReplayMode::Record => matches!(replay, ReplayPolicy::Recordable),
                ReplayMode::Replay => false,
            },
            _ => true,
        };

        // reject disallowed external effects
        if !allowed {
            return Err(Error::ExternalCallForbidden {
                name: spec.name.to_string(),
            });
        }

        Ok(())
    }
}
