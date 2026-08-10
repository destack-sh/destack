mod memory;

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
mod disk;
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
mod segment;

pub(crate) use memory::*;

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub(crate) use disk::*;
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub(crate) use segment::*;
