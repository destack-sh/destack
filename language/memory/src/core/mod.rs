mod error;
#[cfg(not(target_arch = "wasm32"))]
mod fault;

pub use error::*;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use fault::*;
