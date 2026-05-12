#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub mod abi_generated;

pub use abi_generated::*;

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
pub(crate) mod network;
pub(crate) mod notification;
pub(crate) mod permission;
pub(crate) mod power;
mod state;
pub(crate) use state::{PlatformOsState, invalid_data, parse_host_permission_name};
