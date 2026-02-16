#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;
mod handles;
pub mod native;
mod resolve;
pub(crate) mod runtime;
mod snapshot;
mod table;
#[cfg(test)]
mod tests;
pub mod vm;

pub use bindings_generated::*;
pub use handles::*;
pub use snapshot::{
    ResourceDescriptor, ResourceSnapshot, ResourceSnapshotAdapter, ResourceSnapshotPolicy,
};
pub use table::{ResourceEntry, ResourceFinalizer, ResourceKind, ResourceTable};
