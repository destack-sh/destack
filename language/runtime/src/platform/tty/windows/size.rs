use std::mem::MaybeUninit;

use windows_sys::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFO, COORD, GetConsoleScreenBufferInfo, ResizePseudoConsole, SMALL_RECT,
    SetConsoleScreenBufferSize, SetConsoleWindowInfo,
};

use super::core::{
    conpty_error_from_hresult, io_error, tty_binding, tty_handle, validate_console_dimension,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::tty::TtySize;
use crate::platform::tty::core::ensure_out;
use crate::runtime::BindingCallContext;

/// Read terminal size.
///
/// Read one terminal size snapshot for one terminal handle.
/// Pixel fields may be zero when host APIs do not provide pixel metrics.
///
/// # Platform
/// Unix and Windows.
/// Uses TIOCGWINSZ on Unix and console buffer APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.size`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_tty_get_size(
    context: &BindingCallContext,
    out: *mut TtySize,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one pty-backed size cache when present
    let binding = tty_binding(context, handle, "destack.tty.size.getSize")?;
    if let Some(binding) = binding {
        let size = *binding.size.read();

        unsafe {
            out.write(size);
        }

        return Ok(());
    }

    // resolve one console tty handle
    let host_handle = tty_handle(context, handle, "destack.tty.size.getSize")?;

    // query one console screen-buffer snapshot
    let mut info = MaybeUninit::<CONSOLE_SCREEN_BUFFER_INFO>::zeroed();
    let status = unsafe { GetConsoleScreenBufferInfo(host_handle, info.as_mut_ptr()) };
    if status == 0 {
        return Err(io_error(
            "destack.tty.size.getSize",
            "GetConsoleScreenBufferInfo",
            "failed to read console size",
        ));
    }
    let info = unsafe { info.assume_init() };

    // project current console window dimensions into tty size
    let rows = (i32::from(info.srWindow.Bottom) - i32::from(info.srWindow.Top) + 1).max(0) as u32;
    let columns =
        (i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1).max(0) as u32;
    let size = TtySize {
        rows,
        columns,
        x_pixels: 0,
        y_pixels: 0,
    };

    // write size snapshot to output storage
    unsafe {
        out.write(size);
    }

    Ok(())
}

/// Apply terminal size.
///
/// Apply one terminal size to one terminal handle.
/// Resize propagation to attached sessions follows host terminal semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses TIOCSWINSZ on Unix and console size APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.size`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_tty_set_size(
    context: &BindingCallContext,
    handle: resource::TtyHandle,
    size: TtySize,
) -> RuntimeResult<()> {
    // validate one requested window geometry
    let rows = validate_console_dimension(size.rows, "size.rows")?;
    let columns = validate_console_dimension(size.columns, "size.columns")?;

    // resolve one pty-backed size cache when present
    let binding = tty_binding(context, handle, "destack.tty.size.setSize")?;
    if let Some(binding) = binding {
        if let Some(pseudo_console) = binding.pseudo_console {
            let console_size = COORD {
                X: columns,
                Y: rows,
            };
            let status = unsafe { ResizePseudoConsole(pseudo_console, console_size) };
            if status < 0 {
                return Err(conpty_error_from_hresult(
                    "destack.tty.size.setSize",
                    "ResizePseudoConsole",
                    status,
                    "failed to resize pseudo console",
                ));
            }
        }

        *binding.size.write() = TtySize {
            rows: size.rows,
            columns: size.columns,
            x_pixels: 0,
            y_pixels: 0,
        };
        return Ok(());
    }

    // resolve one console tty handle
    let host_handle = tty_handle(context, handle, "destack.tty.size.setSize")?;

    // set one matching screen-buffer geometry
    let buffer_size = COORD {
        X: columns,
        Y: rows,
    };
    let buffer_status = unsafe { SetConsoleScreenBufferSize(host_handle, buffer_size) };
    if buffer_status == 0 {
        return Err(io_error(
            "destack.tty.size.setSize",
            "SetConsoleScreenBufferSize",
            "failed to apply console buffer size",
        ));
    }

    // set one matching visible window rectangle
    let window = SMALL_RECT {
        Left: 0,
        Top: 0,
        Right: columns - 1,
        Bottom: rows - 1,
    };
    let window_status = unsafe { SetConsoleWindowInfo(host_handle, 1, &window) };
    if window_status == 0 {
        return Err(io_error(
            "destack.tty.size.setSize",
            "SetConsoleWindowInfo",
            "failed to apply console window size",
        ));
    }

    Ok(())
}
