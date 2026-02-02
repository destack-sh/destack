#[path = "abi.generated.rs"]
mod abi_generated;
mod snapshot;
mod table;

pub use abi_generated::*;
pub use snapshot::{
    ResourceDescriptor, ResourceSnapshot, ResourceSnapshotAdapter, ResourceSnapshotPolicy,
};
pub use table::{ResourceEntry, ResourceFinalizer, ResourceKind, ResourceTable};
