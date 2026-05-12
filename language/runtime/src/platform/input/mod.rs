#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;

pub(crate) use abi_generated::*;

mod core;
pub(crate) mod host;
mod state;
pub(crate) mod validation;

#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) use core::*;
pub(crate) use state::*;
