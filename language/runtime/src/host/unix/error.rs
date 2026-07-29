use crate::diagnostic::{RuntimeError, io_error_code_from_errno};
use crate::host::HostError;

/// Return the errno pointer for Linux and DragonFly.
#[cfg(any(target_os = "linux", target_os = "dragonfly"))]
fn errno_location() -> *mut libc::c_int {
    // SAFETY: libc returns the thread-local errno storage for the current thread
    unsafe { libc::__errno_location() }
}

/// Return the errno pointer for Apple and FreeBSD targets.
#[cfg(any(target_os = "macos", target_os = "freebsd"))]
fn errno_location() -> *mut libc::c_int {
    // SAFETY: libc returns the thread-local errno storage for the current thread
    unsafe { libc::__error() }
}

/// Return the errno pointer for OpenBSD and NetBSD.
#[cfg(any(target_os = "openbsd", target_os = "netbsd"))]
fn errno_location() -> *mut libc::c_int {
    // SAFETY: libc returns the thread-local errno storage for the current thread
    unsafe { libc::__errno() }
}

/// Read the current errno value.
pub(crate) fn get_errno() -> libc::c_int {
    // SAFETY: errno_location returns valid thread-local storage on supported Unix targets
    unsafe { *errno_location() }
}

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
    RuntimeError::from(HostError::io_with(
        code,
        None,
        Some(errno),
        Some(syscall.to_string()),
        path.map(|path| path.to_string()),
        message,
    ))
    .boxed()
}
