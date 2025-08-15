//! destack.basics.script.run@2025.08.15.1

#![destack::partial(destack.basics.script.run, file)]

#[destack::generated(RunStatus, enum, block)]
/// RunStatus
pub enum RunStatus {
    /// Scheduled for sometime
    SCHEDULED = 2,
    /// Actively running
    RUNNING = 10,
    /// Paused manually
    PAUSED = 21,
    /// Yielded to someone
    YIELDED = 23,
    /// Cancelled before running
    CANCELLED = 51,
    /// Aborted while running
    ABORTED = 52,
    /// Failed due to an error
    FAILED = 53,
    /// Completed successfully
    COMPLETED = 54
}