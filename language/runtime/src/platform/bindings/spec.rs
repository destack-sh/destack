/// Replay behavior for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayPolicy {
    /// Record the call for replay and return replayed values in replay mode.
    Recordable,
    /// Reject the call in deterministic or replay modes.
    Forbidden,
}

/// Effect classification for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectClass {
    /// No observable side effects.
    Pure,
    /// Deterministic effects that do not require external I/O.
    Deterministic,
    /// External side effects governed by replay policy.
    External { replay: ReplayPolicy },
}

/// Metadata describing a runtime binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingDescriptor {
    /// Stable external name for ABI resolution.
    pub name: &'static str,
    /// Effect classification for policy and replay.
    pub effect_class: EffectClass,
}

impl BindingDescriptor {
    /// Create a pure binding descriptor.
    pub const fn pure(name: &'static str) -> Self {
        Self {
            name,
            effect_class: EffectClass::Pure,
        }
    }

    /// Create a deterministic binding descriptor.
    pub const fn deterministic(name: &'static str) -> Self {
        Self {
            name,
            effect_class: EffectClass::Deterministic,
        }
    }

    /// Create a recordable external binding descriptor.
    pub const fn external_recordable(name: &'static str) -> Self {
        Self {
            name,
            effect_class: EffectClass::External {
                replay: ReplayPolicy::Recordable,
            },
        }
    }

    /// Create a forbidden external binding descriptor.
    pub const fn external_forbidden(name: &'static str) -> Self {
        Self {
            name,
            effect_class: EffectClass::External {
                replay: ReplayPolicy::Forbidden,
            },
        }
    }
}
