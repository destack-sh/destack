mod core;
#[cfg(any(unix, windows))]
mod event;
#[cfg(any(unix, windows))]
mod registration;

pub(super) use core::*;
