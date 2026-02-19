#[cfg(any(unix, windows))]
mod basic;
#[cfg(any(unix, windows))]
mod local;
#[cfg(any(unix, windows))]
mod priority;
#[cfg(any(unix, windows))]
mod spawn;
#[cfg(any(unix, windows))]
mod sync;
#[cfg(any(unix, windows))]
mod tests;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
