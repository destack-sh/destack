#[cfg(any(unix, windows))]
mod poll;
#[cfg(any(unix, windows))]
mod tests;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
