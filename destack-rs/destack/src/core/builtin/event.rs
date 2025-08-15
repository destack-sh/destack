//! destack.core.builtin.event@2025.08.15.1

#![destack::partial(destack.core.builtin.event, file)]

#[destack::generated(EventStatus, enum, block)]
/// The consensus status of an Event.
pub enum EventStatus {
    /// Pending application on client
    Pending = 1,
    /// Optimistically staged on client
    Staged = 2,
    /// Successfully applied in system
    Approved = 10,
    /// Skipped and ignored in system
    Skipped = 11,
    /// Could not apply in system
    Failed = 12,
    /// Denied by the system
    Rejected = 13
}