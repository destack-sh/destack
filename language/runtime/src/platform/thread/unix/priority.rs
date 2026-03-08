#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]
use std::mem;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::resource::ThreadHandle;
use crate::platform::thread::{core as core_thread, resource as resource_thread};

use crate::runtime::BindingCallContext;

/// Build one pthread scheduling error from a return code.
fn pthread_error(syscall: &str, code: libc::c_int) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code),
        Some(syscall.to_string()),
        None,
        format!("{syscall} failed: errno {code}"),
    ))
    .boxed()
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

    // read affinity on targets that expose pthread affinity APIs
    #[cfg(target_os = "linux")]
    {
        let mut cpu_set: libc::cpu_set_t = unsafe { mem::zeroed() };
        let rc = unsafe {
            libc::pthread_getaffinity_np(
                resource.native_handle,
                mem::size_of::<libc::cpu_set_t>(),
                &mut cpu_set,
            )
        };
        if rc != 0 {
            return Err(pthread_error("pthread_getaffinity_np", rc));
        }

        let mut mask = 0_u64;
        for cpu in 0..64 {
            let is_set = unsafe { libc::CPU_ISSET(cpu, &cpu_set) };
            if is_set {
                mask |= 1_u64 << cpu;
            }
        }

        unsafe {
            *out = mask;
        }
        Ok(())
    }

    // report unsupported affinity reads on android pthread targets
    #[cfg(target_os = "android")]
    {
        let _ = resource;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.priority.getAffinity",
        ))
        .boxed())
    }

    // report unsupported affinity reads on targets without pthread affinity APIs
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = resource;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.priority.getAffinity",
        ))
        .boxed())
    }
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

    // read scheduling policy and parameters from the host thread
    let mut policy = 0_i32;
    let mut parameters: libc::sched_param = unsafe { mem::zeroed() };
    let rc = unsafe {
        libc::pthread_getschedparam(resource.native_handle, &mut policy, &mut parameters)
    };
    if rc != 0 {
        return Err(pthread_error("pthread_getschedparam", rc));
    }

    unsafe {
        *out = parameters.sched_priority;
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

    // resolve the thread handle resource
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // write affinity on targets that expose pthread affinity APIs
    #[cfg(target_os = "linux")]
    {
        let mut cpu_set: libc::cpu_set_t = unsafe { mem::zeroed() };
        for cpu in 0..64 {
            if (mask & (1_u64 << cpu)) != 0 {
                unsafe {
                    libc::CPU_SET(cpu, &mut cpu_set);
                }
            }
        }

        let rc = unsafe {
            libc::pthread_setaffinity_np(
                resource.native_handle,
                mem::size_of::<libc::cpu_set_t>(),
                &cpu_set,
            )
        };
        if rc != 0 {
            return Err(pthread_error("pthread_setaffinity_np", rc));
        }

        Ok(())
    }

    // report unsupported affinity writes on android pthread targets
    #[cfg(target_os = "android")]
    {
        let _ = resource;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.priority.setAffinity",
        ))
        .boxed())
    }

    // report unsupported affinity writes on targets without pthread affinity APIs
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = resource;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.priority.setAffinity",
        ))
        .boxed())
    }
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

    // read current scheduling policy before setting one new priority
    let mut policy = 0_i32;
    let mut parameters: libc::sched_param = unsafe { mem::zeroed() };
    let get_rc = unsafe {
        libc::pthread_getschedparam(resource.native_handle, &mut policy, &mut parameters)
    };
    if get_rc != 0 {
        return Err(pthread_error("pthread_getschedparam", get_rc));
    }

    // write one new scheduling priority
    parameters.sched_priority = priority;
    let set_rc =
        unsafe { libc::pthread_setschedparam(resource.native_handle, policy, &parameters) };
    if set_rc != 0 {
        return Err(pthread_error("pthread_setschedparam", set_rc));
    }

    Ok(())
}
