#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;
pub(crate) mod core;
pub mod native;
mod os;
#[cfg(test)]
mod tests;
pub mod vm;

pub use crate::platform::resource::{DirectoryHandle, FileHandle};
pub use abi_generated::*;
pub use bindings_generated::*;
