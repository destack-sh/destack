mod core;
#[cfg(test)]
mod tests;
#[cfg(unix)]
pub(crate) mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
pub(crate) mod windows;

pub(crate) use core::*;
