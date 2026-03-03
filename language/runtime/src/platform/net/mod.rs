#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;
pub(crate) mod core;
mod host;
pub mod native;
pub(crate) mod simulation;
mod state;
#[cfg(test)]
mod tests;
pub mod vm;

pub use crate::platform::resource::{ListenerHandle, SocketHandle};
#[allow(unused_imports, unreachable_pub)]
pub use abi_generated::*;
#[allow(unused_imports, unreachable_pub)]
pub use bindings_generated::*;
pub(crate) use state::*;
