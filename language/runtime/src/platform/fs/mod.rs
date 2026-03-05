#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;
pub(crate) mod core;
mod host;
pub mod native;
pub(crate) mod simulation;
#[cfg(windows)]
mod state;
#[cfg(test)]
mod tests;
pub mod vm;

pub use crate::platform::resource::{DirectoryHandle, FileHandle};
pub use abi_generated::*;
pub use bindings_generated::*;
#[cfg(windows)]
pub(crate) use state::*;
