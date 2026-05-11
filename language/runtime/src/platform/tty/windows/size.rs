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

/// Build one console window rectangle from one width and height.
fn console_window(columns: i16, rows: i16) -> SMALL_RECT {
    SMALL_RECT {
        Left: 0,
        Top: 0,
        Right: columns - 1,
        Bottom: rows - 1,
    }
}

/// Read terminal size.
pub(crate) unsafe fn destack_tty_get_size(
    binding: &BindingCallContext,
    out: *mut TtySize,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one pty-backed size cache when present
    let resolved_binding = tty_binding(binding, handle, "destack.tty.size.getSize")?;
    if let Some(resolved_binding) = resolved_binding {
        let size = *resolved_binding.size.read();

        unsafe {
            out.write(size);
        }

        return Ok(());
    }

    // resolve one console tty handle
    let host_handle = tty_handle(binding, handle, "destack.tty.size.getSize")?;

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
pub(crate) unsafe fn destack_tty_set_size(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    size: TtySize,
) -> RuntimeResult<()> {
    // validate one requested window geometry
    let rows = validate_console_dimension(size.rows, "size.rows")?;
    let columns = validate_console_dimension(size.columns, "size.columns")?;

    // resolve one pty-backed size cache when present
    let resolved_binding = tty_binding(binding, handle, "destack.tty.size.setSize")?;
    if let Some(resolved_binding) = resolved_binding {
        if let Some(pseudo_console) = resolved_binding.pseudo_console {
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

        *resolved_binding.size.write() = TtySize {
            rows: size.rows,
            columns: size.columns,
            x_pixels: 0,
            y_pixels: 0,
        };
        return Ok(());
    }

    // resolve one console tty handle
    let host_handle = tty_handle(binding, handle, "destack.tty.size.setSize")?;

    // read the current console geometry to order resize operations safely
    let mut info = MaybeUninit::<CONSOLE_SCREEN_BUFFER_INFO>::zeroed();
    let info_status = unsafe { GetConsoleScreenBufferInfo(host_handle, info.as_mut_ptr()) };
    if info_status == 0 {
        return Err(io_error(
            "destack.tty.size.setSize",
            "GetConsoleScreenBufferInfo",
            "failed to read console size before resize",
        ));
    }
    let info = unsafe { info.assume_init() };

    // shrink the visible window first when the target is smaller in either dimension
    let current_rows = info.srWindow.Bottom - info.srWindow.Top + 1;
    let current_columns = info.srWindow.Right - info.srWindow.Left + 1;
    if rows < current_rows || columns < current_columns {
        let intermediate_rows = rows.min(current_rows);
        let intermediate_columns = columns.min(current_columns);
        let intermediate_window = console_window(intermediate_columns, intermediate_rows);
        let shrink_status = unsafe { SetConsoleWindowInfo(host_handle, 1, &intermediate_window) };
        if shrink_status == 0 {
            return Err(io_error(
                "destack.tty.size.setSize",
                "SetConsoleWindowInfo",
                "failed to shrink console window before resize",
            ));
        }
    }

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
    let window = console_window(columns, rows);
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
