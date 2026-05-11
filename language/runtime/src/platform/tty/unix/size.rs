use std::mem::MaybeUninit;

use super::core::{io_error, tty_descriptor, validate_winsize_dimension};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tty::TtySize;
use crate::platform::tty::core::ensure_out;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Read terminal size.
pub(crate) unsafe fn destack_tty_get_size(
    binding: &BindingCallContext,
    out: *mut TtySize,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one tty descriptor
    let descriptor = tty_descriptor(binding, handle, "destack.tty.size.getSize")?;

    // query one host winsize snapshot
    let mut host_size = MaybeUninit::<libc::winsize>::zeroed();
    let status = unsafe { libc::ioctl(descriptor, libc::TIOCGWINSZ, host_size.as_mut_ptr()) };
    if status < 0 {
        return Err(io_error(
            "destack.tty.size.getSize",
            "ioctl(TIOCGWINSZ)",
            "failed to read tty size",
        ));
    }
    let host_size = unsafe { host_size.assume_init() };

    // project host winsize fields into platform fields
    let size = TtySize {
        rows: host_size.ws_row as u32,
        columns: host_size.ws_col as u32,
        x_pixels: host_size.ws_xpixel as u32,
        y_pixels: host_size.ws_ypixel as u32,
    };

    // write size snapshot to output storage
    unsafe {
        out.write(size);
    }

    Ok(())
}

/// Apply terminal size.
pub(crate) unsafe fn destack_tty_set_size(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    size: TtySize,
) -> RuntimeResult<()> {
    // resolve one tty descriptor
    let descriptor = tty_descriptor(binding, handle, "destack.tty.size.setSize")?;

    // validate and project caller dimensions into winsize
    let rows = validate_winsize_dimension(size.rows, "size.rows")?;
    let columns = validate_winsize_dimension(size.columns, "size.columns")?;
    let x_pixels = u16::try_from(size.x_pixels).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "size.xPixels",
            "size.xPixels exceeds host winsize range",
        ))
        .boxed()
    })?;
    let y_pixels = u16::try_from(size.y_pixels).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "size.yPixels",
            "size.yPixels exceeds host winsize range",
        ))
        .boxed()
    })?;

    let host_size = libc::winsize {
        ws_row: rows,
        ws_col: columns,
        ws_xpixel: x_pixels,
        ws_ypixel: y_pixels,
    };

    // apply one host winsize update
    let status = unsafe { libc::ioctl(descriptor, libc::TIOCSWINSZ, &host_size) };
    if status < 0 {
        return Err(io_error(
            "destack.tty.size.setSize",
            "ioctl(TIOCSWINSZ)",
            "failed to apply tty size",
        ));
    }

    Ok(())
}
