#[cfg(any(unix, windows))]
mod clock;
#[cfg(any(unix, windows))]
mod core;
#[cfg(any(unix, windows))]
mod device;
#[cfg(any(unix, windows))]
mod event;
#[cfg(any(unix, windows))]
mod stream;
#[cfg(any(unix, windows))]
mod tests;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
