//! destack.core.builtin.event@2025.08.15.1

#![destack::partial(destack.core.builtin.event, file)]

#[destack::generated(EventStatus, enum, block)]
/// The consensus status of an Event.
pub enum EventStatus {
    /// Pending application on client
    PENDING = 1,
    /// Optimistically staged on client
    STAGED = 2,
    /// Successfully applied in system
    APPROVED = 10,
    /// Skipped and ignored in system
    SKIPPED = 11,
    /// Could not apply in system
    FAILED = 12,
    /// Denied by the system
    REJECTED = 13
}