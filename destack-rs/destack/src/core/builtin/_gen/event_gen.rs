//! destack.core.builtin.event@2025.08.15.1

#![destack::generated(destack.core.builtin.event, file)]

use crate::EventStatus;

#[destack::generated(EventStatus, Debug, block)]
impl std::fmt::Debug for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStatus::Pending => write!(f, "PENDING"),
            EventStatus::Staged => write!(f, "STAGED"),
            EventStatus::Approved => write!(f, "APPROVED"),
            EventStatus::Skipped => write!(f, "SKIPPED"),
            EventStatus::Failed => write!(f, "FAILED"),
            EventStatus::Rejected => write!(f, "REJECTED"),
        }
    }
}
