#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
mod affinity;
#[path = "bindings.generated.rs"]
mod bindings_generated;
mod handle;
mod kind;
pub mod native;
pub(crate) mod resolve;
mod snapshot;
mod table;
pub mod vm;

pub use affinity::*;
pub(crate) use bindings_generated::*;
pub use handle::*;
pub use kind::*;
pub(crate) use resolve::ensure_resource_affinity;
pub use snapshot::{
    ResourceBacking, ResourceCapture, ResourceImageEntry, ResourcePortability, ResourceProvider,
    ResourceRebinder, ResourceRebinders, ResourceRoute, ResourceSnapshot,
};
pub(crate) use table::ResourceTableSnapshot;
pub use table::{ResourceEntry, ResourceFinalizer, ResourceTable};
