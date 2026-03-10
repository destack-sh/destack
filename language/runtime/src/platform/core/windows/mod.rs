mod com;
mod win32;
mod winsock;

pub(crate) use com::{
    COM_IID_IUNKNOWN, callback_boundary, com_guid_equals, com_non_null_from_raw, com_release_with,
    define_com_callback_vtable, define_com_iunknown_methods,
};
pub(crate) use win32::{
    WaitStatus, decode_wait_for_single_object_status, error_message, io_error, io_error_with_code,
    io_error_with_platform_code, last_error_code, last_wsa_error_code, net_error,
    net_error_with_code, pathbuf_from_utf8, pathbuf_from_utf16, qpc_frequency_hz,
    qpc_hundred_nanos_to_process_nanos, qpc_now_ticks, qpc_process_monotonic_nanos,
    qpc_ticks_to_hundred_nanos, qpc_ticks_to_ns, string_from_utf8, string_from_wide, wide_from_str,
    wide_from_utf8, wide_from_utf16, wide_with_nul,
};
pub(crate) use winsock::ensure_winsock;
