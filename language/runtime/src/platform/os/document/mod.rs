mod core;
#[cfg(all(test, any(target_os = "macos", windows)))]
mod tests;
#[cfg(all(test, target_os = "macos"))]
pub(crate) mod unix;
#[cfg(all(test, windows))]
pub(crate) mod windows;

pub(crate) use core::*;
