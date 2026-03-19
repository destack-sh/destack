mod core;
mod target;
#[cfg(test)]
mod tests;
#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

pub(crate) use core::*;
