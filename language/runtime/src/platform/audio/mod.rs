#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

#[allow(unused_imports, unreachable_pub)]
pub use abi_generated::*;
#[allow(unused_imports, unreachable_pub)]
pub use bindings_generated::*;

mod backend;
mod core;
mod kind;
mod native;
pub(crate) mod simulation;
mod state;
#[cfg(test)]
mod tests;
#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
pub mod vm;
#[cfg(windows)]
mod windows;

pub(crate) use backend::{backend_descriptors, resolve_requested_backend};
pub(crate) use kind::AudioEventKind;
pub(crate) use state::*;
