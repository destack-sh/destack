use super::core::{UnixDescriptorFinalizer, io_error, io_error_with_errno, set_cloexec};

use crate::diagnostic::RuntimeResult;
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::tty::core::{
    close_tty_resource, ensure_out, invalid_file_handle, not_terminal_error,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resolve one file handle into one unix descriptor.
fn file_descriptor(
    binding: &BindingCallContext,
    handle: resource::FileHandle,
    operation: &'static str,
) -> RuntimeResult<libc::c_int> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::File {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| invalid_file_handle(operation))?;

    Ok(descriptor)
}

/// Return whether one descriptor targets a terminal.
fn is_terminal_descriptor(descriptor: libc::c_int, operation: &'static str) -> RuntimeResult<bool> {
    // query host tty state for one descriptor
    let status = unsafe { libc::isatty(descriptor) };
    if status == 1 {
        return Ok(true);
    }

    // map not-a-tty into false and preserve other host errors
    let errno = core_platform::get_errno();
    if errno == 0 || errno == libc::ENOTTY {
        return Ok(false);
    }

    Err(io_error_with_errno(
        operation,
        "isatty",
        errno,
        "failed to query terminal state",
    ))
}

/// Register one duplicated stdio descriptor as a tty handle.
fn register_stdio_tty(
    binding: &BindingCallContext,
    out: *mut resource::TtyHandle,
    descriptor: libc::c_int,
    operation: &'static str,
    label: &'static str,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // reject streams that are not attached to terminals
    let is_terminal = is_terminal_descriptor(descriptor, operation)?;
    if !is_terminal {
        return Err(not_terminal_error(operation));
    }

    // duplicate one descriptor for runtime ownership
    let duplicated = unsafe { libc::dup(descriptor) };
    if duplicated < 0 {
        return Err(io_error(
            operation,
            "dup",
            "failed to duplicate standard stream descriptor",
        ));
    }

    // mark the duplicated descriptor close-on-exec
    if let Err(error) = set_cloexec(
        duplicated,
        operation,
        "failed to set close-on-exec on duplicated descriptor",
    ) {
        unsafe {
            libc::close(duplicated);
        }
        return Err(error);
    }

    // register one tty entry in the resource table
    let entry = ResourceEntry::new(ResourceKind::Tty)
        .with_label(label)
        .with_fd(duplicated)
        .with_finalizer(UnixDescriptorFinalizer {
            descriptor: duplicated,
        });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

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

    // resolve one file descriptor and query host tty state
    let descriptor = file_descriptor(binding, handle, "destack.tty.handle.isTerminalFile")?;
    let is_terminal = is_terminal_descriptor(descriptor, "destack.tty.handle.isTerminalFile")?;

    unsafe {
        out.write(is_terminal);
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
        libc::STDIN_FILENO,
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
        libc::STDOUT_FILENO,
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
        libc::STDERR_FILENO,
        "destack.tty.handle.stdioStderr",
        "tty.stdio.stderr",
    )
}
