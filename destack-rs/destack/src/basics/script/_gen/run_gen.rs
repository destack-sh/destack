//! destack.basics.script.run@2025.08.15.1

#![destack::generated(destack.basics.script.run, file)]

use crate::RunStatus;

#[destack::generated(RunStatus, Debug, block)]
impl std::fmt::Debug for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Scheduled => write!(f, "SCHEDULED"),
            RunStatus::Running => write!(f, "RUNNING"),
            RunStatus::Paused => write!(f, "PAUSED"),
            RunStatus::Yielded => write!(f, "YIELDED"),
            RunStatus::Cancelled => write!(f, "CANCELLED"),
            RunStatus::Aborted => write!(f, "ABORTED"),
            RunStatus::Failed => write!(f, "FAILED"),
            RunStatus::Completed => write!(f, "COMPLETED"),
        }
    }
}
