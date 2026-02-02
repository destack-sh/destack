#[path = "abi.generated.rs"]
mod abi_generated;
mod table;

pub use abi_generated::*;
pub use table::{ResourceEntry, ResourceFinalizer, ResourceKind, ResourceTable};
