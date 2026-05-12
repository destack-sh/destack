#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
pub(crate) mod core;
pub(crate) mod host;
mod state;

pub(crate) use crate::platform::resource::SocketHandle;
pub(crate) use abi_generated::*;
pub(crate) use state::*;
