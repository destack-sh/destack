mod dispatch;
mod foundation;
mod time;

#[cfg(target_os = "macos")]
pub(crate) use dispatch::{DispatchBound, apple_dispatch_queue, apple_serial_dispatch_queue};
#[cfg(target_os = "macos")]
pub(crate) use foundation::{nsdata_to_vec, nsstring_to_string};
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) use time::apple_process_nanos_to_host_time;
pub(crate) use time::{
    apple_host_time_resolution_nanos, apple_host_time_to_process_nanos,
    apple_process_monotonic_nanos,
};
