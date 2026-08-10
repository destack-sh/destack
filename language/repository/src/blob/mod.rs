mod error;
mod memory;
mod store;

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
mod disk;

pub use error::*;
pub use memory::*;
pub use store::*;

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub use disk::*;
