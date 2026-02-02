use super::ReplayEvent;

/// Replay writer interface for deterministic logging.
pub trait ReplayWriter {
    /// Record an event in the replay log.
    fn record_event(&mut self, event: ReplayEvent);
}
