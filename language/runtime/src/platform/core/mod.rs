mod errno;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod win32;
#[cfg(windows)]
mod winsock;

#[cfg(unix)]
pub(crate) use errno::{get_errno, set_errno};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use unix::io_error_with_errno;
#[cfg(unix)]
pub(crate) use unix::{io_error, net_error, writev_all_fd};
#[cfg(windows)]
#[allow(unused_imports)]
pub(crate) use win32::{
    error_message, io_error, io_error_with_code, io_error_with_platform_code, last_error_code,
    last_wsa_error_code, net_error, net_error_with_code, pathbuf_from_utf8, pathbuf_from_utf16,
    string_from_utf8, string_from_wide, wide_from_str, wide_from_utf8, wide_from_utf16,
    write_console_wide, write_file_bytes,
};
#[cfg(windows)]
pub(crate) use winsock::ensure_winsock;
