//! destack.core.common.snapshot@2025.08.15.1

#![destack::partial(destack.core.common.snapshot, file)]

#[destack::generated(SnapshotType, , block)]
/// The type of a Snapshot.
pub enum SnapshotType {
    Full = 10,
    Root = 11,
}

#[destack::generated(SnapshotStatus, , block)]
/// The status of a Snapshot.
pub enum SnapshotStatus {
    /// Under construction
    Creating = 1,
    /// Live and editable
    Active = 10,
    /// Inactive and read-only
    Passive = 50,
}
