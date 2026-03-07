#[path = "abi.generated.rs"]
mod abi_generated;
mod affinity;
#[path = "bindings.generated.rs"]
mod bindings_generated;
mod handle;
mod kind;
pub mod native;
pub(crate) mod resolve;
pub(crate) mod runtime;
mod snapshot;
mod table;
#[cfg(test)]
mod tests;
pub mod vm;

pub use affinity::*;
pub use bindings_generated::*;
pub use handle::*;
pub use kind::*;
pub(crate) use resolve::ensure_resource_affinity;
pub use snapshot::{
    ResourceDescriptor, ResourceSnapshot, ResourceSnapshotAdapter, ResourceSnapshotPolicy,
};
pub use table::{ResourceEntry, ResourceFinalizer, ResourceTable};
