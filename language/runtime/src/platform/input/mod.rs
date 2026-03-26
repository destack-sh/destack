#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

pub(crate) use abi_generated::*;
pub(crate) use bindings_generated::*;

pub(crate) mod clipboard;

mod core;
pub(crate) mod host;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) mod mobile_text;
pub mod native;
pub(crate) mod simulation;
mod state;
#[cfg(any(test, feature = "execution"))]
mod tests;
pub(crate) mod validation;
pub mod vm;

pub(crate) use core::*;
pub(crate) use state::*;
