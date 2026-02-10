#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;
pub(crate) mod core;
pub mod native;
mod native_missing;
mod os;
#[cfg(test)]
mod tests;
pub mod vm;
mod vm_missing;

pub use crate::platform::resource::{ListenerHandle, SocketHandle};
#[allow(unused_imports, unreachable_pub)]
pub use abi_generated::*;
#[allow(unused_imports, unreachable_pub)]
pub use bindings_generated::*;
