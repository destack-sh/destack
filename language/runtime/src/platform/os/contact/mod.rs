mod core;
#[cfg(all(test, any(target_os = "linux", target_os = "macos", windows)))]
mod tests;

pub(crate) use core::*;
