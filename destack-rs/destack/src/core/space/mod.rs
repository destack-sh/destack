//! destack.core.space@2025.08.15.1

#![destack::partial(destack.core.space, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::space::branch::BranchType;
pub use crate::core::space::folder::FolderType;
pub use crate::core::space::snapshot::{SnapshotStatus, SnapshotType};

pub mod _gen;
pub mod branch;
pub mod folder;
pub mod snapshot;
pub mod space;
