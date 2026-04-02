#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

pub use abi_generated::*;
pub(crate) use bindings_generated::*;

pub(crate) mod background;
pub(crate) mod calendar;
pub(crate) mod contact;
pub(crate) mod credentials;
pub(crate) mod document;
pub(crate) mod host;
pub(crate) mod info;
pub(crate) mod intent;
pub(crate) mod lifecycle;
pub(crate) mod location;
pub(crate) mod media;
pub(crate) mod mount;
pub mod native;
pub(crate) mod network;
pub(crate) mod notification;
pub(crate) mod permission;
pub(crate) mod power;
pub(crate) mod simulation;
mod state;
pub mod vm;

#[cfg(any(test, feature = "execution"))]
pub(crate) mod tests;
pub(crate) use state::{PlatformOsState, invalid_data, parse_host_permission_name};
