use crate::platform::bindings::ExecutionMode;
use crate::replay::{ReplayEvent, ReplayHeader, ReplayLog};

/// Replay state for record/replay pipelines.
#[derive(Debug, Clone)]
pub struct ReplayLogState {
    /// Active replay mode.
    mode: ExecutionMode,
    /// Replay log backing store.
    log: ReplayLog,
}

impl ReplayLogState {
    /// Create a replay state with an explicit mode.
    pub fn new(mode: ExecutionMode, header: ReplayHeader) -> Self {
        Self {
            mode,
            log: ReplayLog::new(header),
        }
    }

    /// Return the active replay mode.
    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    /// Return the backing replay log.
    pub fn log(&self) -> &ReplayLog {
        &self.log
    }

    /// Record an event when replay recording is enabled.
    pub fn record_event(&self, event: ReplayEvent) {
        // skip recording when disabled
        if self.mode != ExecutionMode::Record {
            return;
        }

        self.log.record_event(event);
    }

    /// Read the next event when replay is enabled.
    pub fn next_event(&self) -> Option<ReplayEvent> {
        // skip replay when disabled
        if self.mode != ExecutionMode::Replay {
            return None;
        }

        self.log.next_event()
    }
}

impl Default for ReplayLogState {
    fn default() -> Self {
        Self::new(ExecutionMode::Fast, ReplayHeader::default())
    }
}
