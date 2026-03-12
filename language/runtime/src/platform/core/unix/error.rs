use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;
use crate::platform::core::get_errno;
use crate::platform::diagnostic::{io_error_code_from_errno, net_error_code_from_errno};

/// Build an I/O runtime error from the last unix errno value.
pub(crate) fn io_error(syscall: &str, path: Option<&str>) -> Box<RuntimeError> {
    let errno = get_errno();
    io_error_with_errno(syscall, errno, path)
}

/// Build an I/O runtime error from an explicit unix errno value.
pub(crate) fn io_error_with_errno(
    syscall: &str,
    errno: i32,
    path: Option<&str>,
) -> Box<RuntimeError> {
    let message = format!("{syscall} failed: errno {errno}");
    let code = io_error_code_from_errno(errno);
    RuntimeError::from(PlatformError::io_with(
        code,
        None,
        Some(errno),
        Some(syscall.to_string()),
        path.map(|path| path.to_string()),
        message,
    ))
    .boxed()
}

/// Build a network runtime error from the last unix errno value.
pub(crate) fn net_error(syscall: &str) -> Box<RuntimeError> {
    let errno = get_errno();
    net_error_with_errno(syscall, errno)
}

/// Build a network runtime error from an explicit unix errno value.
pub(crate) fn net_error_with_errno(syscall: &str, errno: i32) -> Box<RuntimeError> {
    let message = format!("{syscall} failed: errno {errno}");
    let code = net_error_code_from_errno(errno);
    RuntimeError::from(PlatformError::net_with(
        code,
        None,
        Some(errno),
        Some(syscall.to_string()),
        None,
        None,
        message,
    ))
    .boxed()
}
