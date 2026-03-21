mod core;
mod target;
#[cfg(any(test, feature = "execution"))]
pub(crate) mod tests;
#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

pub(crate) use core::*;
