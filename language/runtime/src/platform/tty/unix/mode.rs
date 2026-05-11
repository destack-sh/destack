use std::mem::MaybeUninit;

use super::core::{host_numeric_to_u64, io_error, tty_descriptor};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tty::TtyMode;
use crate::platform::tty::core::ensure_out;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Apply one raw-mode profile to one termios payload.
fn apply_raw_mode(mode: &mut libc::termios, enabled: bool) {
    // apply one canonical host raw profile when enabled
    if enabled {
        unsafe {
            libc::cfmakeraw(mode);
        }
        return;
    }

    // restore one cooked profile for common interactive terminal behavior
    mode.c_iflag |= libc::BRKINT | libc::ICRNL | libc::IXON;
    mode.c_oflag |= libc::OPOST;
    mode.c_lflag |= libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN;
    mode.c_cc[libc::VMIN] = 1;
    mode.c_cc[libc::VTIME] = 0;
}

/// Read terminal mode flags.
pub(crate) unsafe fn destack_tty_get_mode(
    binding: &BindingCallContext,
    out: *mut TtyMode,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one tty descriptor
    let descriptor = tty_descriptor(binding, handle, "destack.tty.mode.getMode")?;

    // query host termios flags
    let mut host_mode = MaybeUninit::<libc::termios>::uninit();
    let status = unsafe { libc::tcgetattr(descriptor, host_mode.as_mut_ptr()) };
    if status < 0 {
        return Err(io_error(
            "destack.tty.mode.getMode",
            "tcgetattr",
            "failed to read tty mode",
        ));
    }
    let host_mode = unsafe { host_mode.assume_init() };

    // project host mode into platform fields
    let mode = TtyMode {
        input_flags: host_numeric_to_u64(host_mode.c_iflag),
        output_flags: host_numeric_to_u64(host_mode.c_oflag),
        control_flags: host_numeric_to_u64(host_mode.c_cflag),
        local_flags: host_numeric_to_u64(host_mode.c_lflag),
    };

    // write mode snapshot to output storage
    unsafe {
        out.write(mode);
    }

    Ok(())
}

/// Apply terminal mode flags.
pub(crate) unsafe fn destack_tty_set_mode(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    mode: TtyMode,
) -> RuntimeResult<()> {
    // resolve one tty descriptor
    let descriptor = tty_descriptor(binding, handle, "destack.tty.mode.setMode")?;

    // decode host termios flag widths
    let input_flags = libc::tcflag_t::try_from(mode.input_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "mode.inputFlags",
            "mode.inputFlags exceeds host termios range",
        ))
        .boxed()
    })?;
    let output_flags = libc::tcflag_t::try_from(mode.output_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "mode.outputFlags",
            "mode.outputFlags exceeds host termios range",
        ))
        .boxed()
    })?;
    let control_flags = libc::tcflag_t::try_from(mode.control_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "mode.controlFlags",
            "mode.controlFlags exceeds host termios range",
        ))
        .boxed()
    })?;
    let local_flags = libc::tcflag_t::try_from(mode.local_flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "mode.localFlags",
            "mode.localFlags exceeds host termios range",
        ))
        .boxed()
    })?;

    // read current termios payload as mutation baseline
    let mut host_mode = MaybeUninit::<libc::termios>::uninit();
    let get_status = unsafe { libc::tcgetattr(descriptor, host_mode.as_mut_ptr()) };
    if get_status < 0 {
        return Err(io_error(
            "destack.tty.mode.setMode",
            "tcgetattr",
            "failed to read tty mode before update",
        ));
    }
    let mut host_mode = unsafe { host_mode.assume_init() };

    // apply caller-provided flags
    host_mode.c_iflag = input_flags;
    host_mode.c_oflag = output_flags;
    host_mode.c_cflag = control_flags;
    host_mode.c_lflag = local_flags;

    // commit updated termios payload
    let set_status = unsafe { libc::tcsetattr(descriptor, libc::TCSANOW, &host_mode) };
    if set_status < 0 {
        return Err(io_error(
            "destack.tty.mode.setMode",
            "tcsetattr",
            "failed to apply tty mode",
        ));
    }

    Ok(())
}

/// Enable or disable raw terminal mode.
pub(crate) unsafe fn destack_tty_set_raw_mode(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve one tty descriptor
    let descriptor = tty_descriptor(binding, handle, "destack.tty.mode.setRawMode")?;

    // query one host termios payload as mutation baseline
    let mut host_mode = MaybeUninit::<libc::termios>::uninit();
    let get_status = unsafe { libc::tcgetattr(descriptor, host_mode.as_mut_ptr()) };
    if get_status < 0 {
        return Err(io_error(
            "destack.tty.mode.setRawMode",
            "tcgetattr",
            "failed to read tty mode before raw-mode update",
        ));
    }
    let mut host_mode = unsafe { host_mode.assume_init() };

    // apply one raw-mode profile and commit
    apply_raw_mode(&mut host_mode, enabled);
    let set_status = unsafe { libc::tcsetattr(descriptor, libc::TCSANOW, &host_mode) };
    if set_status < 0 {
        return Err(io_error(
            "destack.tty.mode.setRawMode",
            "tcsetattr",
            "failed to apply tty raw-mode update",
        ));
    }

    Ok(())
}
