/// External bindings for platform integration.
pub mod bindings;
/// Time sources and clocks.
pub mod clock;
pub mod host;
/// Randomness and entropy sources.
pub mod random;
/// External resource table and finalizers.
pub mod resources;

pub use bindings::{BindingPolicy, BindingRegistry, BindingSet, DeterminismPolicy, ReplayMode};
pub use clock::Clock;
pub use host::HostContext;
pub use random::Random;
pub use resources::Resources;

/// Platform sources for bindings, time, randomness, and resources.
#[derive(Debug, Default)]
pub struct Platform {
    /// Binding registry for external calls.
    pub bindings: BindingRegistry,
    /// Time sources and clock policies.
    pub clock: Clock,
    /// Randomness and entropy providers.
    pub random: Random,
    /// External resource table and finalizers.
    pub resources: Resources,
}
