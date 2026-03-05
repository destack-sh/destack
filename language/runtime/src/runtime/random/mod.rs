mod host;
pub(crate) mod native;
mod random;
#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
mod r#virtual;
pub(crate) mod vm;
#[cfg(windows)]
mod windows;

pub(crate) use host::HostRandom;
pub use random::*;
pub(crate) use r#virtual::StreamStateDecodeError;
