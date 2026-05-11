#![allow(dead_code)]

use std::os::fd::RawFd;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::io_error_code_from_errno;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::tty::PtyPair;
use crate::platform::tty::core::{
    PTY_RESOURCE_LABEL, TTY_RESOURCE_LABEL, invalid_pty_handle, invalid_tty_handle,
};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Finalizer that closes one unix descriptor.
#[derive(Debug)]
pub(super) struct UnixDescriptorFinalizer {
    /// Descriptor to close.
    pub(super) descriptor: RawFd,
}

impl ResourceFinalizer for UnixDescriptorFinalizer {
    /// Close the descriptor during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.descriptor);
        }
    }
}

/// Build one mapped unix I/O error from errno.
pub(super) fn io_error_with_errno(
    operation: &'static str,
    syscall: &'static str,
    errno: i32,
    message: &str,
) -> Box<RuntimeError> {
    let platform_code = io_error_code_from_errno(errno);
    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(errno),
        Some(operation.to_string()),
        None,
        format!("{syscall} failed: {message}"),
    ))
    .boxed()
}

/// Build one mapped unix I/O error from current errno.
pub(super) fn io_error(
    operation: &'static str,
    syscall: &'static str,
    message: &str,
) -> Box<RuntimeError> {
    let errno = core_platform::get_errno();
    io_error_with_errno(operation, syscall, errno, message)
}

/// Validate one length for host read and write calls.
pub(super) fn validate_buffer_length(length: usize, field: &str) -> RuntimeResult<()> {
    if length > isize::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "buffer length exceeds host addressable range",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one terminal dimension for unix winsize payloads.
pub(super) fn validate_winsize_dimension(value: u32, field: &str) -> RuntimeResult<u16> {
    if value == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "dimension must be greater than zero",
        ))
        .boxed());
    }

    u16::try_from(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "dimension exceeds host winsize range",
        ))
        .boxed()
    })
}

/// Convert one host numeric lane into one runtime u64 value.
pub(super) fn host_numeric_to_u64<T>(value: T) -> u64
where
    T: Into<u64>,
{
    value.into()
}

/// Resolve one tty handle into one unix descriptor.
pub(super) fn tty_descriptor(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    operation: &'static str,
) -> RuntimeResult<RawFd> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Tty {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| invalid_tty_handle(operation))?;

    Ok(descriptor)
}

/// Resolve one pty handle into one unix descriptor.
pub(super) fn pty_descriptor(
    binding: &BindingCallContext,
    handle: resource::PtyHandle,
    operation: &'static str,
) -> RuntimeResult<RawFd> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Pty {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| invalid_pty_handle(operation))?;

    Ok(descriptor)
}

/// Set close-on-exec on one descriptor.
pub(super) fn set_cloexec(
    descriptor: RawFd,
    operation: &'static str,
    field: &'static str,
) -> RuntimeResult<()> {
    let status = unsafe { libc::fcntl(descriptor, libc::F_SETFD, libc::FD_CLOEXEC) };
    if status < 0 {
        return Err(io_error(operation, "fcntl", field));
    }

    Ok(())
}

/// Register one unix pty pair in the resource table.
pub(super) fn register_pty_pair(
    binding: &BindingCallContext,
    controller_descriptor: RawFd,
    worker_descriptor: RawFd,
) -> PtyPair {
    let controller_entry = ResourceEntry::new(ResourceKind::Pty)
        .with_label(PTY_RESOURCE_LABEL)
        .with_fd(controller_descriptor)
        .with_finalizer(UnixDescriptorFinalizer {
            descriptor: controller_descriptor,
        });
    let controller_id = binding.worker().resources.insert(
        binding.world(),
        controller_entry,
        Some(binding.engine()),
    );

    let worker_entry = ResourceEntry::new(ResourceKind::Tty)
        .with_label(TTY_RESOURCE_LABEL)
        .with_fd(worker_descriptor)
        .with_finalizer(UnixDescriptorFinalizer {
            descriptor: worker_descriptor,
        });
    let worker_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), worker_entry, Some(binding.engine()));

    PtyPair {
        controller: resource::PtyHandle(controller_id),
        worker: resource::TtyHandle(worker_id),
    }
}
