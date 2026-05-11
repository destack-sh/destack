/// Result of one runtime scheduler tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickResult {
    /// One unit of runnable work completed.
    Worked,
    /// Virtual time advanced to the next deadline.
    TimeAdvanced,
    /// Background work is still active outside this tick.
    Background,
    /// No runnable work or future deadlines remained.
    Idle,
}

impl TickResult {
    /// Return whether this tick advanced deterministic execution.
    pub const fn is_progress(self) -> bool {
        matches!(self, Self::Worked | Self::TimeAdvanced)
    }
}
