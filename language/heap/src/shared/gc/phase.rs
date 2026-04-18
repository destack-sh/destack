use serde::{Deserialize, Serialize};

/// One live phase of the shared managed collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SharedGcPhase {
    /// No shared collection is currently active.
    #[default]
    Idle,
    /// Shared reachability marking is active.
    Mark,
    /// Shared sweeping is active.
    Sweep,
}
