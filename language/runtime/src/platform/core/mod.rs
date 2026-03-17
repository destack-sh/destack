#![cfg_attr(not(any(unix, windows)), allow(unused_imports))]
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;

#[cfg(target_os = "android")]
pub(crate) mod android;
mod backend;
mod clock;
mod codec;
mod convert;
#[cfg(any(target_os = "linux", target_os = "android"))]
mod dll;
mod errno;
mod error;
#[cfg(unix)]
mod unix;
mod value;
#[cfg(windows)]
mod windows;

pub(crate) use abi_generated::*;
pub(crate) use backend::{
    aggregate_backend_support, backend_support_error, backend_support_from_check,
};
pub(crate) use clock::monotonic_now_ns;
pub(crate) use codec::*;
#[cfg(unix)]
pub(crate) use convert::duration_from_option_ns;
#[cfg(target_os = "macos")]
pub(crate) use convert::u32_to_isize;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) use convert::{option_u64_or_min, option_u64_to_usize_or_min, u32_to_usize};
pub(crate) use convert::{
    option_u64_to_u32, option_u64_to_usize, u32_to_nonzero_usize, u64_to_usize,
    u64_to_usize_with_message, usize_to_u64,
};
#[cfg(target_os = "linux")]
pub(crate) use dll::load_dll_api_named;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use dll::{DynamicLibrary, load_dll_api_bytes, load_library_with_api};
#[cfg(unix)]
pub(crate) use errno::{get_errno, set_errno};
#[cfg(target_os = "linux")]
pub(crate) use error::invalid_state;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) use error::io_busy;
pub(crate) use error::{
    ensure_out, ensure_zero_flags, invalid_argument, io_not_found, io_operation_error,
    io_would_block, not_supported, unsupported_flags,
};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use unix::io_error_with_errno;
#[cfg(target_os = "linux")]
pub(crate) use unix::load_dynamic_symbol_named;
#[cfg(all(unix, not(target_vendor = "apple")))]
pub(crate) use unix::unix_process_monotonic_nanos;
#[cfg(target_vendor = "apple")]
pub(crate) use unix::{apple_host_time_resolution_nanos, apple_process_monotonic_nanos};
#[cfg(target_vendor = "apple")]
pub(crate) use unix::{apple_host_time_to_process_nanos, apple_process_nanos_to_host_time};
#[cfg(target_os = "linux")]
pub(crate) use unix::{c_string_from_str, string_from_c_str};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use unix::{close_dynamic_library, load_dynamic_symbol, open_dynamic_library};
#[cfg(unix)]
pub(crate) use unix::{io_error, net_error};
pub(crate) use value::{NativeAbiCodec, VmAbiCodec};
#[cfg(windows)]
pub(crate) use windows::{
    COM_IID_IUNKNOWN, WaitStatus, callback_boundary, com_guid_equals, com_non_null_from_raw,
    com_release_with, decode_wait_for_single_object_status, define_com_callback_vtable,
    define_com_iunknown_methods, ensure_winsock, error_message, io_error, io_error_with_code,
    last_error_code, last_wsa_error_code, net_error_with_code, pathbuf_from_utf8,
    pathbuf_from_utf16, qpc_hundred_nanos_to_process_nanos, qpc_process_monotonic_nanos,
    qpc_process_nanos_to_hundred_nanos, string_from_utf8, string_from_wide, wide_from_str,
    wide_from_utf8, wide_from_utf16, wide_with_nul,
};
