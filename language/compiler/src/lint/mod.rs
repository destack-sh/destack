#[cfg(not(feature = "lint"))]
mod disabled;
mod error;
#[cfg(feature = "lint")]
mod process;

#[cfg(not(feature = "lint"))]
pub use disabled::*;
pub use error::*;
#[cfg(feature = "lint")]
pub use process::*;
