#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

pub(crate) use abi_generated::*;
pub(crate) use bindings_generated::*;

mod core;
pub(crate) mod host;
pub mod native;
pub(crate) mod simulation;
mod state;
#[cfg(any(test, feature = "execution"))]
pub(crate) mod tests;
pub(crate) mod validation;
pub mod vm;

#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

pub(crate) use core::*;
pub(crate) use state::*;
