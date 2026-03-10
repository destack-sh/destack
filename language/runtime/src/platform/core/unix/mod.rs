#[cfg(target_vendor = "apple")]
mod apple;
#[cfg(not(target_vendor = "apple"))]
mod clock;
mod dynamic;
mod error;

#[cfg(target_vendor = "apple")]
pub(crate) use apple::{
    apple_host_time_resolution_nanos, apple_host_time_to_process_nanos,
    apple_process_monotonic_nanos, apple_process_nanos_to_host_time,
};
#[cfg(not(target_vendor = "apple"))]
pub(crate) use clock::unix_process_monotonic_nanos;
#[cfg(target_os = "linux")]
pub(crate) use dynamic::load_dynamic_symbol_named;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use dynamic::{close_dynamic_library, load_dynamic_symbol, open_dynamic_library};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use error::io_error_with_errno;
pub(crate) use error::{io_error, net_error};
