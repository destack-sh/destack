#[cfg(any(unix, windows))]
mod basic;
#[cfg(any(unix, windows))]
mod event;
#[cfg(any(unix, windows))]
mod monitor;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(any(unix, windows))]
mod window;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
