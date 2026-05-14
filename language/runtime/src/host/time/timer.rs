use serde::{Deserialize, Serialize};

/// Timer deadline clock domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimerClock {
    /// Wall clock affected by host clock changes.
    Wall = 1,
    /// Monotonic clock for elapsed-time deadlines.
    Monotonic = 2,
}
