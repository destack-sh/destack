use super::Snapshot;

/// Callback invoked on snapshot events.
pub type SnapshotCallback = fn(&Snapshot);

/// Snapshot event hooks for runtime integration.
#[derive(Debug, Clone, Default)]
pub struct SnapshotHooks {
    /// Hook invoked after capturing a snapshot.
    pub on_capture: Option<SnapshotCallback>,
    /// Hook invoked after restoring a snapshot.
    pub on_restore: Option<SnapshotCallback>,
}
