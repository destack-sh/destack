#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

#[allow(unused_imports, unreachable_pub)]
pub use abi_generated::*;
#[allow(unused_imports, unreachable_pub)]
pub use bindings_generated::*;

mod credentials;
#[path = "host/mod.rs"]
mod host_impl;
mod info;
pub mod native;
mod power;
pub(crate) mod simulation;
#[cfg(any(target_os = "linux", windows))]
mod state;
#[cfg(test)]
mod tests;
mod unsupported;
pub mod vm;

#[cfg(any(target_os = "linux", windows))]
pub(crate) use state::*;
