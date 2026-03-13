#[cfg(any(unix, windows))]
mod advise;
#[cfg(any(unix, windows))]
mod lock;
#[cfg(any(unix, windows))]
mod map;
#[cfg(any(unix, windows))]
mod protect;
#[cfg(any(unix, windows))]
mod query;
#[cfg(any(unix, windows))]
mod tests;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
