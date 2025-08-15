//! destack.basics.script.run@2025.08.15.1

#![destack::partial(destack.basics.script.run, file)]

#[destack::generated(RunStatus, , block)]
/// RunStatus
pub enum RunStatus {
    /// Scheduled for sometime
    Scheduled = 2,
    /// Actively running
    Running = 10,
    /// Paused manually
    Paused = 21,
    /// Yielded to someone
    Yielded = 23,
    /// Cancelled before running
    Cancelled = 51,
    /// Aborted while running
    Aborted = 52,
    /// Failed due to an error
    Failed = 53,
    /// Completed successfully
    Completed = 54,
}
