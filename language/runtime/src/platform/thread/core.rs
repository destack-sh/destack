use std::sync::Arc;
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::time::Duration;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceEntry, ResourceId, ResourceKind};
use crate::platform::thread::resource::{ThreadLifecycleState, ThreadResource};
use crate::runtime::BindingCallContext;

/// Canonical owner identifier for one host thread.
#[cfg(any(unix, windows))]
#[allow(dead_code)]
pub(crate) type ThreadOwnerId = u64;
/// Canonical owner identifier for one host thread.
#[cfg(not(any(unix, windows)))]
#[allow(dead_code)]
pub(crate) type ThreadOwnerId = std::thread::ThreadId;

/// Sentinel timeout that means wait indefinitely.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
pub(crate) const WAIT_FOREVER: u64 = u64::MAX;

/// Produce one invalid-handle error.
pub(crate) fn invalid_handle_error(field: &str, kind: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        field,
        format!("unknown {kind}"),
    ))
    .boxed()
}

/// Produce one invalid thread lifecycle-state error.
pub(crate) fn invalid_thread_state_error(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value("handle", message)).boxed()
}

/// Produce one would-block error.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
pub(crate) fn io_would_block_error(
    operation: &str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Produce one timeout error.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
pub(crate) fn io_timed_out_error(operation: &str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoTimedOut),
        None,
        None,
        Some(operation.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Produce one parked thread-spawn error.
pub(crate) fn thread_spawn_unavailable_error() -> Box<RuntimeError> {
    // FUGU #Incomplete: implement runtime installed entry handles for thread.spawn and related exported call entrypoints
    RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.start")).boxed()
}

/// Mark one thread handle as being consumed by join or detach.
pub(crate) fn begin_thread_consume(
    resource: &ThreadResource,
    next_state: ThreadLifecycleState,
) -> RuntimeResult<()> {
    let mut lifecycle = resource.lifecycle.lock().map_err(|_| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            None,
            None,
            "thread lifecycle state lock was poisoned",
        ))
        .boxed()
    })?;

    match *lifecycle {
        ThreadLifecycleState::Joinable => {
            *lifecycle = next_state;
            Ok(())
        }
        ThreadLifecycleState::Joining => Err(invalid_thread_state_error(
            "thread handle is already being joined",
        )),
        ThreadLifecycleState::Detaching => Err(invalid_thread_state_error(
            "thread handle is already being detached",
        )),
    }
}

/// Restore one thread handle to the joinable state after a failed consume attempt.
pub(crate) fn reset_thread_consume(resource: &ThreadResource) -> RuntimeResult<()> {
    let mut lifecycle = resource.lifecycle.lock().map_err(|_| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            None,
            None,
            "thread lifecycle state lock was poisoned",
        ))
        .boxed()
    })?;

    *lifecycle = ThreadLifecycleState::Joinable;
    Ok(())
}

/// Convert binding timeout nanoseconds into an optional duration.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
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
    let address = usize::try_from(address).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "address exceeds host pointer width",
        ))
        .boxed()
    })?;

    Ok(address as *const u32)
}

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
    binding: &BindingCallContext,
    kind: ResourceKind,
    label: &str,
    resource: T,
) -> ResourceId {
    let entry = ResourceEntry::new(kind)
        .with_label(label)
        .with_payload(Arc::new(resource));

    binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()))
}

/// Resolve one shared resource payload from the runtime table.
pub(crate) fn resolve_thread_resource<T: Send + Sync + 'static>(
    binding: &BindingCallContext,
    handle: ResourceId,
    field: &str,
    kind: &str,
) -> RuntimeResult<Arc<T>> {
    let resolved = binding.worker().resources.with_entry(handle, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<T>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_handle_error(field, kind))
}

/// Remove one shared resource payload from the runtime table.
pub(crate) fn take_thread_resource<T: Send + Sync + 'static>(
    binding: &BindingCallContext,
    handle: ResourceId,
    field: &str,
    kind: &str,
) -> RuntimeResult<Arc<T>> {
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle, Some(binding.engine()))
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
