#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

pub(crate) use abi_generated::*;
pub(crate) use bindings_generated::*;

pub(crate) mod clipboard;
mod core;
pub(crate) mod credentials;
pub(crate) mod document;
pub(crate) mod host;
pub(crate) mod info;
pub(crate) mod intent;
#[cfg(test)]
pub(crate) mod lifecycle;
pub(crate) mod mount;
pub mod native;
pub(crate) mod network;
#[cfg(test)]
pub(crate) mod permission;
pub(crate) mod power;
pub(crate) mod simulation;
mod state;
pub mod vm;

#[cfg(any(test, feature = "execution"))]
pub(crate) mod tests;
pub(crate) use state::*;
