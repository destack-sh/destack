#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

pub(crate) use abi_generated::*;
pub(crate) use bindings_generated::*;

pub mod native;
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
pub(crate) mod options;
pub(crate) mod simulation;
mod state;
#[cfg(any(test, feature = "affinity"))]
pub(crate) mod tests;
#[cfg(unix)]
mod unix;
mod unsupported;
pub mod vm;
#[cfg(windows)]
mod windows;

pub(crate) use state::*;
