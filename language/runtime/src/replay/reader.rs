use super::ReplayEvent;

/// Replay reader interface for deterministic replay.
pub trait ReplayReader {
    /// Read the next event in the replay log.
    fn next_event(&mut self) -> Option<ReplayEvent>;
}
