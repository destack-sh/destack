#[cfg(target_os = "macos")]
mod macos;
mod target;
#[cfg(not(target_os = "macos"))]
mod unsupported;

pub(crate) use target::*;
