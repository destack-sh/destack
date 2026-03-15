#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

pub(crate) use abi_generated::*;
pub(crate) use bindings_generated::*;

mod credentials;
#[path = "host/mod.rs"]
mod host_impl;
mod info;
pub mod native;
mod power;
pub(crate) mod simulation;
mod state;
#[cfg(test)]
mod tests;
pub mod vm;

pub(crate) use state::*;
