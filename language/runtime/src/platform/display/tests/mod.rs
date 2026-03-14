#[cfg(any(unix, windows))]
#[macro_use]
mod macros;
#[cfg(any(unix, windows))]
pub(crate) mod backend;
#[cfg(any(unix, windows))]
pub(crate) mod basic;
#[cfg(any(unix, windows))]
pub(crate) mod event;
#[cfg(any(unix, windows))]
pub(crate) mod monitor;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(any(unix, windows))]
pub(crate) mod window;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
