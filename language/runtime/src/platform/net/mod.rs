#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
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
pub(crate) use abi_generated::*;
pub(crate) use bindings_generated::*;
pub(crate) use state::*;
