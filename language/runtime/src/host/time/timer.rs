use serde::{Deserialize, Serialize};

/// Timer clock domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimerClock {
    /// Wall clock.
    Wall = 1,
    /// Monotonic clock.
    Monotonic = 2,
}
