#[cfg(feature = "optimize")]
pub mod analyses;
#[cfg(feature = "optimize")]
pub(crate) mod common;
#[cfg(not(feature = "optimize"))]
mod disabled;
mod error;
#[cfg(feature = "optimize")]
pub mod passes;
#[cfg(feature = "optimize")]
pub mod pipeline;
#[cfg(feature = "optimize")]
mod provide;
#[cfg(feature = "optimize")]
mod state;
mod warning;

#[cfg(feature = "optimize")]
pub use analyses::*;
#[cfg(feature = "optimize")]
pub use common::*;
pub use error::*;
#[cfg(feature = "optimize")]
pub use pipeline::*;
#[cfg(feature = "optimize")]
pub(in crate::optimize) use state::*;
pub use warning::*;
