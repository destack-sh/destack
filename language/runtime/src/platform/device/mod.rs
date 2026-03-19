#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

pub(crate) use abi_generated::*;
pub(crate) use bindings_generated::*;
mod bluetooth;
mod camera;
mod host;
pub mod midi;
pub mod native;
mod serial;
pub(crate) mod simulation;
mod state;
#[cfg(test)]
mod tests;
#[allow(dead_code)]
mod unsupported;
mod usb;
pub mod vm;

pub(crate) use state::*;
