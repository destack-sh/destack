//! destack.core.common.snapshot@2025.08.15.1

#![destack::generated(destack.core.common.snapshot, file)]

use crate::SnapshotStatus;
use crate::SnapshotType;

#[destack::generated(SnapshotType, Debug, block)]
impl std::fmt::Debug for SnapshotType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotType::Full => write!(f, "FULL"),
            SnapshotType::Root => write!(f, "ROOT"),
        }
    }
}

#[destack::generated(SnapshotStatus, Debug, block)]
impl std::fmt::Debug for SnapshotStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotStatus::Creating => write!(f, "CREATING"),
            SnapshotStatus::Active => write!(f, "ACTIVE"),
            SnapshotStatus::Passive => write!(f, "PASSIVE"),
        }
    }
}
