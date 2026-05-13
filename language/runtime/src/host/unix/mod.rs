#[cfg(target_os = "macos")]
mod apple;
#[cfg(not(target_os = "macos"))]
mod clock;
mod error;

#[cfg(target_os = "macos")]
pub(crate) use apple::apple_process_monotonic_nanos;
#[cfg(not(target_os = "macos"))]
pub(crate) use clock::unix_process_monotonic_nanos;
pub(crate) use error::io_error;
