#[cfg(any(unix, windows))]
mod core;
#[cfg(any(target_os = "linux", target_os = "android"))]
mod message;
#[cfg(any(unix, windows))]
mod pipe;
#[cfg(any(unix, windows))]
mod shared_memory;
#[cfg(any(unix, windows))]
mod sync;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(unix)]
mod unix;
#[cfg(any(unix, windows))]
mod unsupported;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
