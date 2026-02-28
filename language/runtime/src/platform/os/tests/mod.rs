#[cfg(any(unix, windows))]
mod core;
#[cfg(any(unix, windows))]
mod credentials;
#[cfg(any(unix, windows))]
mod host;
#[cfg(any(unix, windows))]
mod info;
#[cfg(any(unix, windows))]
mod power;
#[cfg(any(unix, windows))]
mod tests;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
