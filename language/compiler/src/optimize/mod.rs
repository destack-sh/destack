#[cfg(feature = "optimize")]
pub mod analyses;
#[cfg(feature = "optimize")]
pub mod common;
#[cfg(not(feature = "optimize"))]
mod disabled;
mod error;
#[cfg(feature = "optimize")]
pub mod passes;
#[cfg(feature = "optimize")]
pub mod pipeline;
#[cfg(feature = "optimize")]
mod process;
mod warning;

#[cfg(feature = "optimize")]
pub use analyses::*;
#[cfg(feature = "optimize")]
pub use common::*;
#[cfg(not(feature = "optimize"))]
pub use disabled::*;
pub use error::*;
#[cfg(feature = "optimize")]
pub use pipeline::*;
#[cfg(feature = "optimize")]
pub use process::*;
pub use warning::*;
