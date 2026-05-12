#![cfg_attr(
    not(any(target_os = "android", target_os = "linux", windows)),
    allow(unused_imports)
)]
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;

pub(crate) use abi_generated::*;

mod backend;
mod core;
mod host;
mod kind;
mod state;
#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

pub(crate) use backend::{backend_descriptors, resolve_requested_backend};
pub(crate) use kind::AudioEventKind;
pub(crate) use state::*;
