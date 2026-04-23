use serde::{Deserialize, Serialize};

/// One live phase of the shared heap collector.
#[repr(u8)]
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

impl SharedGcPhase {
    /// Return the atomic representation for this phase.
    pub(crate) const fn bits(self) -> u8 {
        self as u8
    }

    /// Decode one atomic phase byte.
    pub(crate) fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Idle,
            1 => Self::Mark,
            2 => Self::Sweep,
            _ => panic!("invalid shared gc phase: {bits}"),
        }
    }
}
