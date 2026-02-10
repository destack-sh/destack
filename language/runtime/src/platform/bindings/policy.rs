use std::collections::BTreeSet;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::{BindingDescriptor, BindingEffectMask};

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

/// Policy configuration for external bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingPolicy {
    /// Execution mode for external calls.
    mode: ExecutionMode,
    /// Allowed binding effects for this policy.
    allowed: BindingEffectMask,
    /// Optional allowlist of exact capability names.
    capabilities: Option<BTreeSet<String>>,
}

impl BindingPolicy {
    /// Create a binding policy from an execution mode.
    pub const fn new(mode: ExecutionMode) -> Self {
        let allowed = allowed_effects_for_mode(mode);
        Self {
            mode,
            allowed,
            capabilities: None,
        }
    }

    /// Create a binding policy with an explicit capability allowlist.
    pub fn with_capabilities(
        mode: ExecutionMode,
        capabilities: impl IntoIterator<Item = String>,
    ) -> Self {
        // normalize the capability allowlist
        let capabilities = capabilities
            .into_iter()
            .filter(|capability| !capability.is_empty())
            .collect::<BTreeSet<_>>();

        // store allowlist only when it has entries
        let capabilities = if capabilities.is_empty() {
            None
        } else {
            Some(capabilities)
        };

        Self {
            mode,
            allowed: allowed_effects_for_mode(mode),
            capabilities,
        }
    }

    /// Replace the capability allowlist for this policy.
    pub fn set_capabilities(&mut self, capabilities: impl IntoIterator<Item = String>) {
        let capabilities = capabilities
            .into_iter()
            .filter(|capability| !capability.is_empty())
            .collect::<BTreeSet<_>>();

        self.capabilities = if capabilities.is_empty() {
            None
        } else {
            Some(capabilities)
        };
    }

    /// Return the execution mode.
    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    /// Validate a binding descriptor against policy.
    #[inline]
    pub fn check(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        if !self.allowed.allows(spec.effect_mask) {
            return Err(RuntimeError::PolicyViolation {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        // check required capabilities with exact matching only
        if let Some(capabilities) = &self.capabilities {
            for capability in spec.requires {
                if capabilities.contains(*capability) {
                    continue;
                }

                return Err(RuntimeError::CapabilityViolation {
                    name: spec.name.to_string(),
                    capability: capability.to_string(),
                }
                .boxed());
            }
        }

        Ok(())
    }
}

impl Default for BindingPolicy {
    fn default() -> Self {
        Self::new(ExecutionMode::Fast)
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
