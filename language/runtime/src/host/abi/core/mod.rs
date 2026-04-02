mod module;

#[cfg(feature = "generator")]
pub use module::*;
#[cfg(not(feature = "generator"))]
pub use module::*;
