use serde::{Deserialize, Serialize};

/// One stored phase of the shared heap collector.
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
        match self {
            Self::Idle => 0,
            Self::Mark => 1,
            Self::Sweep => 2,
        }
    }

    /// Decode one atomic phase byte.
    pub(crate) fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Idle,
            1 => Self::Mark,
            2 => Self::Sweep,
            _ => {
                debug_assert!(bits <= 2, "invalid shared gc phase byte");

                Self::Idle
            }
        }
    }
}
