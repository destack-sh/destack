mod convert;
#[cfg(any(target_os = "linux", target_os = "android"))]
mod dll;
mod errno;
mod error;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::sync::OnceLock;
use std::time::Instant;

#[cfg(unix)]
pub(crate) use convert::duration_from_option_ns;
#[cfg(unix)]
#[allow(unused_imports)]
pub(crate) use convert::u32_to_nonzero_usize;
#[allow(unused_imports)]
pub(crate) use convert::{
    option_u64_or_min, option_u64_to_u32, option_u64_to_usize, option_u64_to_usize_or_min,
    u32_to_isize, u32_to_usize, u64_to_usize, u64_to_usize_with_message, usize_to_u64,
};
#[cfg(target_os = "linux")]
pub(crate) use dll::load_dll_api_named;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use dll::{DynamicLibrary, load_dll_api_bytes, load_library_with_api};
#[cfg(unix)]
pub(crate) use errno::{get_errno, set_errno};
#[cfg(unix)]
#[allow(unused_imports)]
pub(crate) use error::invalid_state;
#[cfg(unix)]
pub(crate) use error::unknown_handle;
#[allow(unused_imports)]
pub(crate) use error::{
    ensure_out, ensure_zero_flags, invalid_argument, io_busy, io_not_found, io_operation_error,
    io_would_block, not_supported, unsupported_flags,
};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use unix::io_error_with_errno;
#[cfg(target_os = "linux")]
pub(crate) use unix::load_dynamic_symbol_named;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use unix::{close_dynamic_library, load_dynamic_symbol, open_dynamic_library};
#[cfg(unix)]
pub(crate) use unix::{io_error, net_error};
#[cfg(windows)]
#[allow(unused_imports)]
pub(crate) use windows::{
    COM_IID_IUNKNOWN, WaitStatus, callback_boundary, com_guid_equals, com_non_null_from_raw,
    com_release_with, decode_wait_for_single_object_status, define_com_callback_vtable,
    define_com_iunknown_methods, ensure_winsock, error_message, io_error, io_error_with_code,
    io_error_with_platform_code, last_error_code, last_wsa_error_code, net_error,
    net_error_with_code, pathbuf_from_utf8, pathbuf_from_utf16, qpc_frequency_hz, qpc_now_ns,
    qpc_now_ticks, qpc_ticks_to_ns, string_from_utf8, string_from_wide, wide_from_str,
    wide_from_utf8, wide_from_utf16, wide_with_nul,
};

/// Return one process-monotonic timestamp in nanoseconds.
#[cfg(any(unix, windows))]
#[allow(dead_code)]
pub(crate) fn monotonic_now_ns() -> u64 {
    static MONO_EPOCH: OnceLock<Instant> = OnceLock::new();

    let elapsed = MONO_EPOCH.get_or_init(Instant::now).elapsed();
    elapsed.as_nanos().min(u64::MAX as u128) as u64
}
