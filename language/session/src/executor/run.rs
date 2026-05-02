use parking_lot::Mutex;

use super::task::Task;
use crate::SessionError;

/// Id for one session run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RunId(pub u32);

impl std::fmt::Display for RunId {
    /// Format this run id for progress output.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

/// One top-level artifact executor run.
#[derive(Debug)]
pub(super) struct Run {
    /// The id for this session run.
    id: RunId,
    /// The root tasks this caller is waiting for.
    roots: Vec<Task>,
    /// The first infrastructure error seen by any worker.
    error: Mutex<Option<SessionError>>,
}

impl Run {
    /// Create one artifact executor run.
    pub(super) fn new(id: RunId, roots: Vec<Task>) -> Self {
        Self {
            id,
            roots,
            error: Mutex::new(None),
        }
    }

    /// Return this session run id.
    pub(super) fn id(&self) -> RunId {
        self.id
    }

    /// Return the root tasks this caller needs.
    pub(super) fn roots(&self) -> &[Task] {
        &self.roots
    }

    /// Return true when one worker has aborted this run.
    pub(super) fn is_aborted(&self) -> bool {
        self.error.lock().is_some()
    }

    /// Record the first infrastructure error for this run.
    pub(super) fn abort(&self, error: SessionError) {
        let mut existing_error = self.error.lock();

        // keep the first error as the run cause
        if existing_error.is_none() {
            *existing_error = Some(error);
        }
    }

    /// Take the first infrastructure error for this run.
    pub(super) fn take_error(&self) -> Option<SessionError> {
        self.error.lock().take()
    }
}
