#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ThreadHandle;
use crate::platform::thread::{core as core_thread, resource as resource_thread};
use crate::platform::{PlatformError, PlatformErrorCode, core as core_platform};
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_CALL_NOT_IMPLEMENTED, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED,
};
use windows_sys::Win32::System::SystemInformation::GROUP_AFFINITY;
use windows_sys::Win32::System::Threading::{
    GetThreadGroupAffinity, GetThreadPriority, SetThreadAffinityMask, SetThreadPriority,
};
use windows_sys::Win32::System::WindowsProgramming::THREAD_PRIORITY_ERROR_RETURN;

use crate::runtime::BindingCallContext;

/// Build one thread-priority error from the last Win32 error.
fn thread_priority_error(syscall: &str) -> Box<RuntimeError> {
    let errno = core_platform::last_error_code();
    let code = match errno as u32 {
        ERROR_ACCESS_DENIED => PlatformErrorCode::IoPermissionDenied,
        ERROR_INVALID_PARAMETER => PlatformErrorCode::IoInvalidData,
        ERROR_NOT_SUPPORTED | ERROR_CALL_NOT_IMPLEMENTED => PlatformErrorCode::NotSupported,
        _ => PlatformErrorCode::Io,
    };
    core_platform::io_error_with_platform_code(syscall, errno, code)
}

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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the thread handle resource
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // query the host thread affinity mask
    let mut affinity = GROUP_AFFINITY {
        Mask: 0,
        Group: 0,
        Reserved: [0, 0, 0],
    };
    let rc = unsafe { GetThreadGroupAffinity(resource.native_handle, &mut affinity) };
    if rc == 0 {
        return Err(thread_priority_error("GetThreadGroupAffinity"));
    }

    let mask = affinity.Mask as u64;
    unsafe {
        *out = mask;
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut i32,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the thread handle resource
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // query the host thread priority
    let priority = unsafe { GetThreadPriority(resource.native_handle) };
    if priority == THREAD_PRIORITY_ERROR_RETURN as i32 {
        return Err(thread_priority_error("GetThreadPriority"));
    }

    unsafe {
        *out = priority;
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: ThreadHandle,
    mask: u64,
) -> RuntimeResult<()> {
    // validate the affinity mask
    if mask == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mask",
            "affinity mask must not be zero",
        ))
        .boxed());
    }

    // validate affinity mask width on 32-bit windows hosts
    if std::mem::size_of::<usize>() < std::mem::size_of::<u64>() && mask > (usize::MAX as u64) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mask",
            "affinity mask exceeds host platform width",
        ))
        .boxed());
    }

    // resolve the thread handle resource
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // apply the host thread affinity mask
    let rc = unsafe { SetThreadAffinityMask(resource.native_handle, mask as usize) };
    if rc == 0 {
        return Err(thread_priority_error("SetThreadAffinityMask"));
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: ThreadHandle,
    priority: i32,
) -> RuntimeResult<()> {
    // resolve the thread handle resource
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // apply the host thread priority
    let rc = unsafe { SetThreadPriority(resource.native_handle, priority) };
    if rc == 0 {
        return Err(thread_priority_error("SetThreadPriority"));
    }

    Ok(())
}
