use std::os::fd::RawFd;

use super::core::{register_pty_pair, set_cloexec, validate_winsize_dimension};

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use crate::platform::core as core_platform;
use crate::platform::tty::PtyPair;
use crate::platform::tty::core::{close_pty_resource, ensure_out, validate_zero_flags};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Return whether unix pty host APIs are available on this target.
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
const fn pty_available() -> bool {
    true
}

/// Return whether unix pty host APIs are available on this target.
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
const fn pty_available() -> bool {
    false
}

/// Open one pty pair through host openpty.
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn open_pty_pair(rows: u16, columns: u16) -> RuntimeResult<(RawFd, RawFd)> {
    // initialize output descriptors and winsize payload
    let mut controller = -1;
    let mut worker = -1;
    let mut host_size = libc::winsize {
        ws_row: rows,
        ws_col: columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    // open one pty controller and worker descriptor pair
    let status = unsafe {
        libc::openpty(
            &mut controller,
            &mut worker,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut host_size as *mut libc::winsize,
        )
    };
    if status < 0 {
        return Err(core_platform::io_error("openpty", None));
    }

    Ok((controller, worker))
}

/// Open one pty pair through host openpty.
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
fn open_pty_pair(_rows: u16, _columns: u16) -> RuntimeResult<(RawFd, RawFd)> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.pty.open")).boxed())
}

/// Close one pseudo-terminal controller.
pub(crate) unsafe fn destack_tty_pty_close(
    binding: &BindingCallContext,
    handle: resource::PtyHandle,
) -> RuntimeResult<()> {
    close_pty_resource(binding, handle, "destack.tty.pty.close")
}

/// Open one pseudo-terminal pair.
pub(crate) unsafe fn destack_tty_pty_open(
    binding: &BindingCallContext,
    out: *mut PtyPair,
    rows: u32,
    columns: u32,
    flags: u32,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // reject unsupported open flags
    validate_zero_flags(flags, "flags")?;

    // reject hosts without pty support
    if !pty_available() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.tty.pty.open")).boxed(),
        );
    }

    // validate dimensions and open one host pair
    let rows = validate_winsize_dimension(rows, "rows")?;
    let columns = validate_winsize_dimension(columns, "columns")?;
    let (controller_descriptor, worker_descriptor) = open_pty_pair(rows, columns)?;

    // apply close-on-exec to both descriptors
    if let Err(error) = set_cloexec(
        controller_descriptor,
        "destack.tty.pty.open",
        "failed to set close-on-exec on controller descriptor",
    ) {
        unsafe {
            libc::close(controller_descriptor);
            libc::close(worker_descriptor);
        }
        return Err(error);
    }
    if let Err(error) = set_cloexec(
        worker_descriptor,
        "destack.tty.pty.open",
        "failed to set close-on-exec on worker descriptor",
    ) {
        unsafe {
            libc::close(controller_descriptor);
            libc::close(worker_descriptor);
        }
        return Err(error);
    }

    // register resources and write the returned pair
    let pair = register_pty_pair(binding, controller_descriptor, worker_descriptor);
    unsafe {
        out.write(pair);
    }

    Ok(())
}
