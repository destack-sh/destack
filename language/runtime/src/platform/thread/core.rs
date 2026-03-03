use std::sync::Arc;
use std::time::Duration;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::core::{
    io_operation_error, u64_to_usize_with_message, unknown_handle,
    unsupported_flags as unsupported_flags_helper,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceEntry, ResourceId, ResourceKind};
use crate::runtime::BindingCallContext;

/// Sentinel timeout that means wait indefinitely.
pub(crate) const WAIT_FOREVER: u64 = u64::MAX;

/// Canonical resource kind for thread-domain runtime resources.
const THREAD_RESOURCE_KIND: ResourceKind = ResourceKind::Thread;

/// Produce one invalid-handle error.
pub(crate) fn invalid_handle_error(field: &str, kind: &str) -> Box<RuntimeError> {
    unknown_handle(field, kind)
}

/// Produce one unsupported-flags error.
pub(crate) fn unsupported_flags_error(field: &str, flags: u32) -> Box<RuntimeError> {
    unsupported_flags_helper(field, flags)
}

/// Produce one thread-deadlock error.
pub(crate) fn thread_deadlock_error(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::ThreadDeadlock),
        message,
    ))
    .boxed()
}

/// Produce one would-block error.
pub(crate) fn io_would_block_error(
    operation: &str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    io_operation_error(operation, Some(PlatformErrorCode::IoWouldBlock), message)
}

/// Produce one timeout error.
pub(crate) fn io_timed_out_error(operation: &str, message: impl Into<String>) -> Box<RuntimeError> {
    io_operation_error(operation, Some(PlatformErrorCode::IoTimedOut), message)
}

/// Produce one permission-denied error.
pub(crate) fn io_permission_denied_error(
    operation: &str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    io_operation_error(
        operation,
        Some(PlatformErrorCode::IoPermissionDenied),
        message,
    )
}

/// Convert binding timeout nanoseconds into an optional duration.
pub(crate) fn timeout_from_ns(timeoutns: u64) -> Option<Duration> {
    if timeoutns == WAIT_FOREVER {
        return None;
    }

    Some(Duration::from_nanos(timeoutns))
}

/// Convert one address argument into one checked aligned u32 word pointer.
#[allow(dead_code)]
pub(crate) fn checked_u32_word_pointer(address: u64, field: &str) -> RuntimeResult<*const u32> {
    // reject null addresses explicitly
    if address == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "address must not be zero",
        ))
        .boxed());
    }

    // reject misaligned futex or wait-on-address words
    if !address.is_multiple_of(std::mem::size_of::<u32>() as u64) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "address must be aligned to 4 bytes",
        ))
        .boxed());
    }

    // reject values that do not fit the host pointer width
    let address = u64_to_usize_with_message(address, field, "address exceeds host pointer width")?;

    Ok(address as *const u32)
}

/// Canonical owner identifier for one host thread.
#[cfg(any(unix, windows))]
#[allow(dead_code)]
pub(crate) type ThreadOwnerId = u64;
/// Canonical owner identifier for one host thread.
#[cfg(not(any(unix, windows)))]
#[allow(dead_code)]
pub(crate) type ThreadOwnerId = std::thread::ThreadId;

/// Return the current host thread owner identifier.
#[cfg(unix)]
#[allow(dead_code)]
pub(crate) fn current_thread_owner_id() -> ThreadOwnerId {
    unsafe { libc::pthread_self() as u64 }
}

/// Return the current host thread owner identifier.
#[cfg(windows)]
#[allow(dead_code)]
pub(crate) fn current_thread_owner_id() -> ThreadOwnerId {
    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() as u64 }
}

/// Return the current host thread owner identifier.
#[cfg(not(any(unix, windows)))]
#[allow(dead_code)]
pub(crate) fn current_thread_owner_id() -> ThreadOwnerId {
    std::thread::current().id()
}

/// Insert one thread-domain resource payload into the runtime table.
pub(crate) fn insert_thread_resource<T: Send + Sync + 'static>(
    context: &BindingCallContext,
    label: &str,
    resource: T,
) -> ResourceId {
    let entry = ResourceEntry::new(THREAD_RESOURCE_KIND)
        .with_label(label)
        .with_payload(Arc::new(resource));

    context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()))
}

/// Resolve one shared resource payload from the runtime table.
pub(crate) fn resolve_thread_resource<T: Send + Sync + 'static>(
    context: &BindingCallContext,
    handle: ResourceId,
    field: &str,
    kind: &str,
) -> RuntimeResult<Arc<T>> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(handle, |entry| entry.payload_cloned::<Arc<T>>());

    resolved
        .flatten()
        .ok_or_else(|| invalid_handle_error(field, kind))
}

/// Remove one shared resource payload from the runtime table.
pub(crate) fn take_thread_resource<T: Send + Sync + 'static>(
    context: &BindingCallContext,
    handle: ResourceId,
    field: &str,
    kind: &str,
) -> RuntimeResult<Arc<T>> {
    let Some(entry) = context
        .runtime()
        .resources
        .remove(handle, Some(context.engine()))
    else {
        return Err(invalid_handle_error(field, kind));
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_handle_error(field, kind));
    };

    match payload.downcast::<Arc<T>>() {
        Ok(resource) => Ok(*resource),
        Err(_) => Err(invalid_handle_error(field, kind)),
    }
}
