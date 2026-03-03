#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;
mod handle;
pub mod native;
mod resolve;
pub(crate) mod runtime;
mod snapshot;
mod table;
#[cfg(test)]
mod tests;
pub mod vm;

pub use bindings_generated::*;
pub use handle::*;
#[cfg(windows)]
pub(crate) use resolve::require_payload_with;
pub(crate) use resolve::{
    require_payload, resolve_payload, with_any_entry, with_entry, with_entry_mut, with_payload,
};
pub use snapshot::{
    ResourceDescriptor, ResourceSnapshot, ResourceSnapshotAdapter, ResourceSnapshotPolicy,
};
pub use table::{ResourceEntry, ResourceFinalizer, ResourceKind, ResourceTable};
