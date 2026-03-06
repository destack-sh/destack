#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

#[allow(unused_imports, unreachable_pub)]
pub use abi_generated::*;
#[allow(unused_imports, unreachable_pub)]
pub use bindings_generated::*;

mod host;
pub mod native;
pub(crate) mod simulation;
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
mod state;
#[cfg(test)]
mod tests;
mod unsupported;
pub mod vm;

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
pub(crate) use state::*;
