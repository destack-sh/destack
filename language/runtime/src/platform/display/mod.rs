#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

#[allow(unused_imports, unreachable_pub)]
pub use abi_generated::*;
#[allow(unused_imports, unreachable_pub)]
pub use bindings_generated::*;

pub mod native;
mod options;
pub(crate) mod simulation;
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
mod state;
#[cfg(any(test, target_os = "macos"))]
pub(crate) mod tests;
#[cfg(unix)]
mod unix;
mod unsupported;
pub mod vm;
#[cfg(windows)]
mod windows;

pub(crate) use options::*;
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
pub(crate) use state::*;
