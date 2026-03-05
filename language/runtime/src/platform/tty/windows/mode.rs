use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode, SetConsoleMode,
};

use super::core::{io_error, tty_binding, tty_handle};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tty::TtyMode;
use crate::platform::tty::core::ensure_out;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Decode one tty mode payload into one Win32 console mode value.
fn decode_console_mode(mode: TtyMode) -> RuntimeResult<u32> {
    if mode.input_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mode.inputFlags",
            "mode.inputFlags is not supported on Windows tty mode",
        ))
        .boxed());
    }

    if mode.output_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mode.outputFlags",
            "mode.outputFlags is not supported on Windows tty mode",
        ))
        .boxed());
    }

    if mode.control_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mode.controlFlags",
            "mode.controlFlags is not supported on Windows tty mode",
        ))
        .boxed());
    }

    u32::try_from(mode.local_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "mode.localFlags",
            "mode.localFlags exceeds Win32 console mode range",
        ))
        .boxed()
    })
}

/// Apply one raw-mode profile to one Win32 console-mode payload.
fn apply_raw_mode(mode: u32, enabled: bool) -> u32 {
    let raw_disable_mask = ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;
    if enabled {
        return mode & !raw_disable_mask;
    }

    mode | raw_disable_mask
}

/// Read terminal mode flags.
///
/// Read one terminal mode snapshot for one terminal handle.
/// Mode fields are projected from host terminal APIs.
/// Field-level behavior is host-specific, especially for non-POSIX backends.
///
/// # Platform
/// Unix and Windows.
/// Uses termios get attributes on Unix and GetConsoleMode on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_tty_get_mode(
    binding: &BindingCallContext,
    out: *mut TtyMode,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one pty-backed mode cache when present
    let resolved_binding = tty_binding(binding, handle, "destack.tty.mode.getMode")?;
    if let Some(resolved_binding) = resolved_binding {
        let mode = *resolved_binding.mode.read();

        unsafe {
            out.write(mode);
        }

        return Ok(());
    }

    // resolve one console tty handle
    let host_handle = tty_handle(binding, handle, "destack.tty.mode.getMode")?;

    // query one console mode snapshot
    let mut console_mode = 0u32;
    let status = unsafe { GetConsoleMode(host_handle, &mut console_mode) };
    if status == 0 {
        return Err(io_error(
            "destack.tty.mode.getMode",
            "GetConsoleMode",
            "failed to read console mode",
        ));
    }

    // map windows console bits into tty mode payload
    let mode = TtyMode {
        input_flags: 0,
        output_flags: 0,
        control_flags: 0,
        local_flags: u64::from(console_mode),
    };

    // write mode snapshot to output storage
    unsafe {
        out.write(mode);
    }

    Ok(())
}

/// Apply terminal mode flags.
///
/// Apply one terminal mode snapshot to one terminal handle.
/// Mode transition timing and unsupported bits follow host API behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses termios set attributes on Unix and SetConsoleMode on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_tty_set_mode(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    mode: TtyMode,
) -> RuntimeResult<()> {
    // decode supported windows mode bits
    let console_mode = decode_console_mode(mode)?;

    // resolve one pty-backed mode cache when present
    let resolved_binding = tty_binding(binding, handle, "destack.tty.mode.setMode")?;
    if let Some(resolved_binding) = resolved_binding {
        *resolved_binding.mode.write() = TtyMode {
            input_flags: 0,
            output_flags: 0,
            control_flags: 0,
            local_flags: u64::from(console_mode),
        };
        return Ok(());
    }

    // resolve one console tty handle
    let host_handle = tty_handle(binding, handle, "destack.tty.mode.setMode")?;

    // apply one console mode update
    let status = unsafe { SetConsoleMode(host_handle, console_mode) };
    if status == 0 {
        return Err(io_error(
            "destack.tty.mode.setMode",
            "SetConsoleMode",
            "failed to apply console mode",
        ));
    }

    Ok(())
}

/// Enable or disable raw terminal mode.
///
/// Apply one host-defined raw-mode profile for one terminal handle.
/// This maps to cfmakeraw-style behavior on Unix and console-mode toggles on Windows.
///
/// # Platform
/// Unix and Windows.
/// Uses cfmakeraw plus tcsetattr on Unix and SetConsoleMode profile updates on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_tty_set_raw_mode(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve one pty-backed mode cache when present
    let resolved_binding = tty_binding(binding, handle, "destack.tty.mode.setRawMode")?;
    if let Some(resolved_binding) = resolved_binding {
        let current = decode_console_mode(*resolved_binding.mode.read())?;
        let next = apply_raw_mode(current, enabled);
        *resolved_binding.mode.write() = TtyMode {
            input_flags: 0,
            output_flags: 0,
            control_flags: 0,
            local_flags: u64::from(next),
        };
        return Ok(());
    }

    // resolve one console tty handle
    let host_handle = tty_handle(binding, handle, "destack.tty.mode.setRawMode")?;

    // query one baseline console mode before mutation
    let mut current = 0u32;
    let get_status = unsafe { GetConsoleMode(host_handle, &mut current) };
    if get_status == 0 {
        return Err(io_error(
            "destack.tty.mode.setRawMode",
            "GetConsoleMode",
            "failed to read console mode before raw-mode update",
        ));
    }

    // apply one raw-mode profile to the console mode bits
    let next = apply_raw_mode(current, enabled);
    let set_status = unsafe { SetConsoleMode(host_handle, next) };
    if set_status == 0 {
        return Err(io_error(
            "destack.tty.mode.setRawMode",
            "SetConsoleMode",
            "failed to apply console raw-mode update",
        ));
    }

    Ok(())
}
