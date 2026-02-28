#![allow(dead_code)]

use std::os::windows::io::RawHandle;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{NativeStringRef, PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resource label used for pipe endpoints.
pub(super) const PIPE_RESOURCE_LABEL: &str = "ipc.pipe";
/// Resource label used for shared-memory objects.
pub(super) const SHARED_MEMORY_RESOURCE_LABEL: &str = "ipc.shared_memory";
/// Resource label used for semaphore objects.
pub(super) const SEMAPHORE_RESOURCE_LABEL: &str = "ipc.semaphore";

/// Finalizer that closes one windows handle.
#[derive(Debug)]
pub(super) struct WindowsHandleFinalizer {
    /// Handle to close.
    pub(super) handle: HANDLE,
}

impl ResourceFinalizer for WindowsHandleFinalizer {
    /// Close the handle during resource cleanup.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        if self.handle == 0 || self.handle == INVALID_HANDLE_VALUE {
            return;
        }

        unsafe {
            CloseHandle(self.handle);
        }
    }
}

/// Validate one out-pointer argument.
pub(super) fn ensure_out<T>(out: *mut T, field: &'static str) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(field)).boxed());
    }

    Ok(())
}

/// Build one invalid-argument runtime error.
pub(super) fn invalid_argument(
    field: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(field, message.into())).boxed()
}

/// Build one not-supported runtime error.
pub(super) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Build one mapped windows I/O error.
pub(super) fn io_error(syscall: &'static str) -> Box<RuntimeError> {
    core_platform::io_error(syscall)
}

/// Build one timed-out runtime error.
pub(super) fn timed_out(operation: &'static str, message: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoTimedOut),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

/// Validate one flag word for the currently supported subset.
pub(super) fn ensure_zero_flags(flags: u32, field: &'static str) -> RuntimeResult<()> {
    if flags != 0 {
        return Err(invalid_argument(
            field,
            "only flags=0 is currently supported",
        ));
    }

    Ok(())
}

/// Decode one native name into one Windows wide string.
pub(super) fn wide_name(name: NativeStringRef, field: &'static str) -> RuntimeResult<Vec<u16>> {
    let name = unsafe { name.as_str()? };
    if name.is_empty() {
        return Err(invalid_argument(field, "name must not be empty"));
    }

    core_platform::wide_from_str(field, name)
}

/// Validate one windows handle payload.
pub(super) fn validate_handle(
    handle: HANDLE,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<HANDLE> {
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(invalid_argument(
            field,
            format!("{operation} received one invalid windows handle"),
        ));
    }

    Ok(handle)
}

/// Resolve one pipe handle into one windows handle.
pub(super) fn pipe_handle(
    context: &BindingCallContext,
    handle: resource::PipeHandle,
    operation: &'static str,
) -> RuntimeResult<HANDLE> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Pipe {
                return None;
            }

            entry.handle().map(|value| value as HANDLE)
        })
        .flatten()
        .ok_or_else(|| {
            invalid_argument(
                "handle",
                format!("{operation} expected one valid pipe handle"),
            )
        })?;

    validate_handle(resolved, "handle", operation)
}

/// Resolve one shared-memory handle into one windows handle.
pub(super) fn shared_memory_handle(
    context: &BindingCallContext,
    handle: resource::SharedMemoryHandle,
    operation: &'static str,
) -> RuntimeResult<HANDLE> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::SharedMemory {
                return None;
            }

            entry.handle().map(|value| value as HANDLE)
        })
        .flatten()
        .ok_or_else(|| {
            invalid_argument(
                "handle",
                format!("{operation} expected one valid shared-memory handle"),
            )
        })?;

    validate_handle(resolved, "handle", operation)
}

/// Resolve one semaphore handle into one windows handle.
pub(super) fn semaphore_handle(
    context: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    operation: &'static str,
) -> RuntimeResult<HANDLE> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Semaphore {
                return None;
            }

            entry.handle().map(|value| value as HANDLE)
        })
        .flatten()
        .ok_or_else(|| {
            invalid_argument(
                "handle",
                format!("{operation} expected one valid semaphore handle"),
            )
        })?;

    validate_handle(resolved, "handle", operation)
}

/// Convert one windows handle into one raw-handle payload.
fn as_raw_handle(handle: HANDLE) -> RawHandle {
    handle as RawHandle
}

/// Register one pipe endpoint handle in the runtime resource table.
pub(super) fn register_pipe_handle(
    context: &BindingCallContext,
    handle: HANDLE,
) -> resource::PipeHandle {
    let entry = ResourceEntry::new(ResourceKind::Pipe)
        .with_label(PIPE_RESOURCE_LABEL)
        .with_handle(as_raw_handle(handle))
        .with_finalizer(WindowsHandleFinalizer { handle });
    let resource_id = context.runtime().resources.insert(entry);

    resource::PipeHandle(resource_id)
}

/// Register one shared-memory handle in the runtime resource table.
pub(super) fn register_shared_memory_handle(
    context: &BindingCallContext,
    handle: HANDLE,
) -> resource::SharedMemoryHandle {
    let entry = ResourceEntry::new(ResourceKind::SharedMemory)
        .with_label(SHARED_MEMORY_RESOURCE_LABEL)
        .with_handle(as_raw_handle(handle))
        .with_finalizer(WindowsHandleFinalizer { handle });
    let resource_id = context.runtime().resources.insert(entry);

    resource::SharedMemoryHandle(resource_id)
}

/// Register one semaphore handle in the runtime resource table.
pub(super) fn register_semaphore_handle(
    context: &BindingCallContext,
    handle: HANDLE,
) -> resource::SemaphoreHandle {
    let entry = ResourceEntry::new(ResourceKind::Semaphore)
        .with_label(SEMAPHORE_RESOURCE_LABEL)
        .with_handle(as_raw_handle(handle))
        .with_finalizer(WindowsHandleFinalizer { handle });
    let resource_id = context.runtime().resources.insert(entry);

    resource::SemaphoreHandle(resource_id)
}

/// Convert one timeout in nanoseconds into one wait timeout in milliseconds.
pub(super) fn timeout_to_wait_milliseconds(timeout_ns: u64) -> u32 {
    if timeout_ns == u64::MAX {
        return u32::MAX;
    }

    let milliseconds = timeout_ns.div_ceil(1_000_000);
    if milliseconds > u64::from(u32::MAX - 1) {
        return u32::MAX - 1;
    }

    milliseconds as u32
}
