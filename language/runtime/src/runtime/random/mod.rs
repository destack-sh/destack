mod host;
mod random;
#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
mod r#virtual;
#[cfg(windows)]
mod windows;

pub(crate) use host::HostRandom;
pub use random::*;
