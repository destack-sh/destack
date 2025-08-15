//! destack.core.common.snapshot@2025.08.15.1

#![destack::partial(destack.core.common.snapshot, file)]

#[destack::generated(SnapshotType, enum, block)]
/// The type of a Snapshot.
pub enum SnapshotType {
    FULL = 10,
    ROOT = 11
}

#[destack::generated(SnapshotStatus, enum, block)]
/// The status of a Snapshot.
pub enum SnapshotStatus {
    /// Under construction
    CREATING = 1,
    /// Live and editable
    ACTIVE = 10,
    /// Inactive and read-only
    PASSIVE = 50
}