use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{COORD, CreatePseudoConsole, HPCON};
use windows_sys::Win32::System::Pipes::CreatePipe;

use super::core::{
    conpty_error_from_hresult, io_error, register_pty_pair, validate_console_dimension,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::tty::core::{close_pty_resource, ensure_out, validate_zero_flags};
use crate::platform::tty::{PtyPair, TtySize};
use crate::runtime::BindingCallContext;

/// Close one raw handle when it is valid.
fn close_handle_maybe(handle: HANDLE) {
    if handle != 0 && handle != INVALID_HANDLE_VALUE {
        unsafe {
            CloseHandle(handle);
        }
    }
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

    // validate requested terminal geometry
    let rows_i16 = validate_console_dimension(rows, "rows")?;
    let columns_i16 = validate_console_dimension(columns, "columns")?;

    // create one pipe for tty writes into the pseudo console input
    let mut pseudo_input_read: HANDLE = 0;
    let mut worker_write: HANDLE = 0;
    let input_pipe = unsafe {
        CreatePipe(
            &mut pseudo_input_read,
            &mut worker_write,
            std::ptr::null(),
            0,
        )
    };
    if input_pipe == 0 {
        return Err(io_error(
            "destack.tty.pty.open",
            "CreatePipe",
            "failed to create pseudo console input pipe",
        ));
    }

    // create one pipe for pseudo console output into tty reads
    let mut worker_read: HANDLE = 0;
    let mut pseudo_output_write: HANDLE = 0;
    let output_pipe = unsafe {
        CreatePipe(
            &mut worker_read,
            &mut pseudo_output_write,
            std::ptr::null(),
            0,
        )
    };
    if output_pipe == 0 {
        close_handle_maybe(pseudo_input_read);
        close_handle_maybe(worker_write);
        return Err(io_error(
            "destack.tty.pty.open",
            "CreatePipe",
            "failed to create pseudo console output pipe",
        ));
    }

    // create one windows pseudo console from the pipe pair
    let size = COORD {
        X: columns_i16,
        Y: rows_i16,
    };
    let mut pseudo_console: HPCON = 0;
    let status = unsafe {
        CreatePseudoConsole(
            size,
            pseudo_input_read,
            pseudo_output_write,
            0,
            &mut pseudo_console,
        )
    };
    if status < 0 {
        close_handle_maybe(pseudo_input_read);
        close_handle_maybe(worker_write);
        close_handle_maybe(worker_read);
        close_handle_maybe(pseudo_output_write);
        return Err(conpty_error_from_hresult(
            "destack.tty.pty.open",
            "CreatePseudoConsole",
            status,
            "failed to create pseudo console",
        ));
    }

    // close endpoints now owned by the pseudo console
    close_handle_maybe(pseudo_input_read);
    close_handle_maybe(pseudo_output_write);

    // register the controller and worker resources
    let initial_size = TtySize {
        rows,
        columns,
        x_pixels: 0,
        y_pixels: 0,
    };
    let pair = register_pty_pair(
        binding,
        pseudo_console,
        worker_read,
        worker_write,
        initial_size,
    );
    unsafe {
        out.write(pair);
    }

    Ok(())
}
