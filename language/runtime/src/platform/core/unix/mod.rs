#[cfg(target_vendor = "apple")]
mod apple;
#[cfg(not(target_vendor = "apple"))]
mod clock;
mod dynamic;
mod error;
#[cfg(target_os = "linux")]
mod string;
mod sync;

#[cfg(target_os = "macos")]
pub(crate) use apple::{
    DispatchBound, apple_dispatch_queue, apple_serial_dispatch_queue, nsdata_to_vec,
    nsstring_to_string,
};
#[cfg(target_vendor = "apple")]
pub(crate) use apple::{apple_host_time_resolution_nanos, apple_process_monotonic_nanos};
#[cfg(target_vendor = "apple")]
pub(crate) use apple::{apple_host_time_to_process_nanos, apple_process_nanos_to_host_time};
#[cfg(not(target_vendor = "apple"))]
pub(crate) use clock::unix_process_monotonic_nanos;
#[cfg(target_os = "linux")]
pub(crate) use dynamic::load_dynamic_symbol_named;
#[cfg(unix)]
pub(crate) use dynamic::{close_dynamic_library, load_dynamic_symbol, open_dynamic_library};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use error::io_error_with_errno;
pub(crate) use error::{io_error, net_error};
#[cfg(target_os = "linux")]
pub(crate) use string::{c_string_from_str, string_from_c_str};
pub(crate) use sync::{UnixSemaphoreWaitStatus, unix_semaphore_wait_timed};
