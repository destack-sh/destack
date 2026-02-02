#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;
pub mod native;
pub mod vm;

pub use crate::platform::resource::{DirectoryHandle, FileHandle};
pub use abi_generated::*;
pub use bindings_generated::*;
