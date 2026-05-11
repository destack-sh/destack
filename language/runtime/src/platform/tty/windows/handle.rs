use windows_sys::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_INVALID_HANDLE, HANDLE,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Console::{
    GetConsoleMode, GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::core::{WindowsHandleFinalizer, io_error_with_code};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::tty::core::{
    close_tty_resource, ensure_out, invalid_file_handle, not_terminal_error,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resolve one file handle into one raw windows handle.
fn file_handle(
    binding: &BindingCallContext,
    handle: resource::FileHandle,
    operation: &'static str,
) -> RuntimeResult<HANDLE> {
    let resolved = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::File {
                return None;
            }

            entry.handle().map(|value| value as HANDLE)
        })
        .flatten()
        .ok_or_else(|| invalid_file_handle(operation))?;

    Ok(resolved)
}

/// Return whether one windows handle targets a console terminal.
fn is_console_handle(handle: HANDLE, operation: &'static str) -> RuntimeResult<bool> {
    // query one console mode payload for the handle
    let mut mode = 0u32;
    let status = unsafe { GetConsoleMode(handle, &mut mode) };
    if status != 0 {
        return Ok(true);
    }

    // map invalid-handle state to false and preserve other host errors
    let code = core_platform::last_error_code() as u32;
    if code == ERROR_INVALID_HANDLE {
        return Ok(false);
    }

    Err(io_error_with_code(
        operation,
        "GetConsoleMode",
        code,
        "failed to query terminal state",
    ))
}

/// Register one duplicated standard stream as a tty handle.
fn register_stdio_tty(
    binding: &BindingCallContext,
    out: *mut resource::TtyHandle,
    std_handle: u32,
    operation: &'static str,
    label: &'static str,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one process standard stream handle
    let source = unsafe { GetStdHandle(std_handle) };
    if source == 0 || source == INVALID_HANDLE_VALUE {
        return Err(io_error_with_code(
            operation,
            "GetStdHandle",
            core_platform::last_error_code() as u32,
            "failed to resolve standard stream handle",
        ));
    }

    // duplicate one standard stream handle for runtime ownership
    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = 0;
    let duplicated_status = unsafe {
        DuplicateHandle(
            process,
            source,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if duplicated_status == 0 || duplicated == 0 || duplicated == INVALID_HANDLE_VALUE {
        return Err(io_error_with_code(
            operation,
            "DuplicateHandle",
            core_platform::last_error_code() as u32,
            "failed to duplicate standard stream handle",
        ));
    }

    // reject standard streams that are not console terminals
    let is_console = match is_console_handle(duplicated, operation) {
        Ok(is_console) => is_console,
        Err(error) => {
            unsafe {
                CloseHandle(duplicated);
            }

            return Err(error);
        }
    };
    if !is_console {
        unsafe {
            CloseHandle(duplicated);
        }
        return Err(not_terminal_error(operation));
    }

    // register one tty resource entry
    let entry = ResourceEntry::new(ResourceKind::Tty)
        .with_label(label)
        .with_handle(duplicated as _)
        .with_finalizer(WindowsHandleFinalizer { handle: duplicated });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::TtyHandle(resource_id));
    }

    Ok(())
}

/// Close one terminal handle.
pub(crate) unsafe fn destack_tty_close(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    close_tty_resource(binding, handle, "destack.tty.handle.close")
}

/// Return whether one file handle is attached to a terminal.
pub(crate) unsafe fn destack_tty_is_terminal_file(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one file handle and query console-mode support
    let handle = file_handle(binding, handle, "destack.tty.handle.isTerminalFile")?;
    let is_console = is_console_handle(handle, "destack.tty.handle.isTerminalFile")?;

    unsafe {
        out.write(is_console);
    }

    Ok(())
}

/// Open one standard input terminal handle.
pub(crate) unsafe fn destack_tty_stdio_stdin(
    binding: &BindingCallContext,
    out: *mut resource::TtyHandle,
) -> RuntimeResult<()> {
    register_stdio_tty(
        binding,
        out,
        STD_INPUT_HANDLE,
        "destack.tty.handle.stdioStdin",
        "tty.stdio.stdin",
    )
}

/// Open one standard output terminal handle.
pub(crate) unsafe fn destack_tty_stdio_stdout(
    binding: &BindingCallContext,
    out: *mut resource::TtyHandle,
) -> RuntimeResult<()> {
    register_stdio_tty(
        binding,
        out,
        STD_OUTPUT_HANDLE,
        "destack.tty.handle.stdioStdout",
        "tty.stdio.stdout",
    )
}

/// Open one standard error terminal handle.
pub(crate) unsafe fn destack_tty_stdio_stderr(
    binding: &BindingCallContext,
    out: *mut resource::TtyHandle,
) -> RuntimeResult<()> {
    register_stdio_tty(
        binding,
        out,
        STD_ERROR_HANDLE,
        "destack.tty.handle.stdioStderr",
        "tty.stdio.stderr",
    )
}
