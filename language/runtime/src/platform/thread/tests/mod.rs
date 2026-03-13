#[cfg(any(unix, windows))]
mod local;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(any(unix, windows))]
mod wait;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
