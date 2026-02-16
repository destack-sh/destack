#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::thread::{bindings_generated as bindings, core as core_thread};
use crate::platform::{NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::thread::ThreadOptions;
/// Read thread affinity mask.
///
/// Read one thread CPU affinity mask.
/// Affinity mask width and normalization are host-architecture dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses sched affinity APIs on Unix and GetThreadGroupAffinity on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.priority`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_get_affinity(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.getAffinity",
    ))
    .boxed())
}

/// Read thread priority.
///
/// Read one thread priority value from host scheduler state.
/// Priority value normalization is runtime-defined per host.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread scheduling APIs on Unix and GetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.priority`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_get_priority(
    context: &RuntimeCallContext,
    out: *mut i32,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.getPriority",
    ))
    .boxed())
}

/// Set thread affinity mask.
///
/// Bind one thread to a CPU affinity mask.
/// Affinity mask semantics are host scheduler-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses sched affinity APIs on Unix and SetThreadAffinityMask on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.priority`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_set_affinity(
    context: &RuntimeCallContext,
    handle: resource::ThreadHandle,
    mask: u64,
) -> RuntimeResult<()> {
    let _ = (handle, mask);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.setAffinity",
    ))
    .boxed())
}

/// Set thread priority.
///
/// Set one thread priority value using host scheduler controls.
/// Priority range and interpretation are host-specific.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread scheduling APIs on Unix and SetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.priority`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_set_priority(
    context: &RuntimeCallContext,
    handle: resource::ThreadHandle,
    priority: i32,
) -> RuntimeResult<()> {
    let _ = (handle, priority);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.thread.priority.setPriority",
    ))
    .boxed())
}
