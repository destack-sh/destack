use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// Build an I/O runtime error from the last unix errno value.
pub(crate) fn io_error(syscall: &str, path: Option<&str>) -> Box<RuntimeError> {
    let errno = super::get_errno();
    io_error_with_errno(syscall, errno, path)
}

/// Build an I/O runtime error from an explicit unix errno value.
pub(crate) fn io_error_with_errno(
    syscall: &str,
    errno: i32,
    path: Option<&str>,
) -> Box<RuntimeError> {
    let message = format!("{syscall} failed: errno {errno}");
    RuntimeError::from(PlatformError::io_with(
        None,
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
    let errno = super::get_errno();
    net_error_with_errno(syscall, errno)
}

/// Build a network runtime error from an explicit unix errno value.
pub(crate) fn net_error_with_errno(syscall: &str, errno: i32) -> Box<RuntimeError> {
    let message = format!("{syscall} failed: errno {errno}");
    RuntimeError::from(PlatformError::net_with(
        None,
        None,
        Some(errno),
        Some(syscall.to_string()),
        None,
        None,
        message,
    ))
    .boxed()
}

/// Write a full set of iovec buffers to a unix file descriptor.
pub(crate) fn writev_all_fd(fd: libc::c_int, buffers: &mut [libc::iovec]) -> RuntimeResult<()> {
    let mut start = 0;
    while start < buffers.len() {
        let rc = unsafe {
            libc::writev(
                fd,
                buffers[start..].as_ptr(),
                (buffers.len() - start) as i32,
            )
        };
        if rc < 0 {
            let errno = super::get_errno();
            if errno == libc::EINTR {
                continue;
            }
            return Err(io_error_with_errno("writev", errno, None));
        }
        let mut remaining = rc as usize;
        while remaining > 0 && start < buffers.len() {
            let current = buffers[start].iov_len;
            if remaining < current {
                let base = buffers[start].iov_base as *mut u8;
                let advanced = unsafe { base.add(remaining) } as *mut libc::c_void;
                buffers[start].iov_base = advanced;
                buffers[start].iov_len = current - remaining;
                remaining = 0;
            } else {
                remaining -= current;
                start += 1;
            }
        }
    }
    Ok(())
}
