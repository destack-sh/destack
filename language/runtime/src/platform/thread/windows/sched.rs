#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ThreadHandle;
use crate::platform::thread::{
    ThreadCpu, ThreadCpuSet, core as core_thread, resource as resource_thread,
};
use crate::platform::{PlatformError, PlatformErrorCode, core as core_platform};
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_CALL_NOT_IMPLEMENTED, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED,
};
use windows_sys::Win32::System::SystemInformation::GROUP_AFFINITY;
use windows_sys::Win32::System::Threading::{
    GetThreadGroupAffinity, GetThreadPriority, SetThreadGroupAffinity, SetThreadPriority,
};
use windows_sys::Win32::System::WindowsProgramming::THREAD_PRIORITY_ERROR_RETURN;

use crate::runtime::BindingCallContext;

/// Build one thread-scheduling error from the last Win32 error.
fn thread_priority_error(syscall: &str) -> Box<RuntimeError> {
    let errno = core_platform::last_error_code();
    let code = match errno as u32 {
        ERROR_ACCESS_DENIED => PlatformErrorCode::IoPermissionDenied,
        ERROR_INVALID_PARAMETER => PlatformErrorCode::IoInvalidData,
        ERROR_NOT_SUPPORTED | ERROR_CALL_NOT_IMPLEMENTED => PlatformErrorCode::NotSupported,
        _ => PlatformErrorCode::Io,
    };
    let message = core_platform::error_message(syscall, errno);
    RuntimeError::from(PlatformError::io_with(
        Some(code),
        None,
        Some(errno),
        Some(syscall.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Read thread CPU affinity.
pub(crate) unsafe fn destack_thread_get_affinity(
    binding: &BindingCallContext,
    out: *mut ThreadCpuSet,
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

    let mut cpus = Vec::new();
    for cpu in 0..usize::BITS as usize {
        let bit = 1usize << cpu;
        if (affinity.Mask & bit) != 0 {
            let cpu = u16::try_from(cpu).map_err(|_| {
                RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoInvalidData),
                    None,
                    None,
                    Some("GetThreadGroupAffinity".to_string()),
                    None,
                    "host cpu index exceeds thread affinity ABI range",
                ))
                .boxed()
            })?;
            cpus.push(ThreadCpu {
                group: affinity.Group,
                cpu,
            });
        }
    }

    let value = ThreadCpuSet {
        cpus: binding.store_array(cpus),
    };
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read thread priority.
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

/// Set thread CPU affinity.
pub(crate) unsafe fn destack_thread_set_affinity(
    binding: &BindingCallContext,
    handle: ThreadHandle,
    cpus: ThreadCpuSet,
) -> RuntimeResult<()> {
    let cpus = unsafe { cpus.cpus.as_slice()? };

    // validate the affinity set
    if cpus.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "cpus",
            "thread affinity set must not be empty",
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

    // build one group-affinity mask from the requested cpu set
    let mut affinity = GROUP_AFFINITY {
        Mask: 0,
        Group: cpus[0].group,
        Reserved: [0, 0, 0],
    };
    for cpu in cpus {
        if cpu.group != affinity.Group {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "cpus",
                "windows thread affinity must target one processor group",
            ))
            .boxed());
        }

        let bit_index = usize::from(cpu.cpu);
        if bit_index >= usize::BITS as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "cpus",
                format!(
                    "cpu index {} is out of range for one processor group",
                    cpu.cpu
                ),
            ))
            .boxed());
        }

        affinity.Mask |= 1usize << bit_index;
    }

    // apply the host thread affinity mask
    let rc =
        unsafe { SetThreadGroupAffinity(resource.native_handle, &affinity, std::ptr::null_mut()) };
    if rc == 0 {
        return Err(thread_priority_error("SetThreadGroupAffinity"));
    }

    Ok(())
}

/// Set thread priority.
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
