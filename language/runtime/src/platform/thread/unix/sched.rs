#![allow(clippy::missing_safety_doc)]
use std::mem;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::resource::ThreadHandle;
#[cfg(target_os = "linux")]
use crate::platform::thread::ThreadCpu;
use crate::platform::thread::{ThreadCpuSet, core as core_thread, resource as resource_thread};

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

        let mut cpus = Vec::new();
        let cpu_set_size = std::mem::size_of::<libc::cpu_set_t>() * 8;
        for cpu in 0..cpu_set_size {
            let is_set = unsafe { libc::CPU_ISSET(cpu, &cpu_set) };
            if is_set {
                let cpu = u16::try_from(cpu).map_err(|_| {
                    RuntimeError::from(PlatformError::io_with(
                        None,
                        None,
                        None,
                        Some("pthread_getaffinity_np".to_string()),
                        None,
                        "host cpu index exceeds thread affinity ABI range",
                    ))
                    .boxed()
                })?;
                cpus.push(ThreadCpu { group: 0, cpu });
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

    // report unsupported affinity reads on android pthread targets
    #[cfg(target_os = "android")]
    {
        let _ = resource;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sched.getAffinity",
        ))
        .boxed())
    }

    // report unsupported affinity reads on targets without pthread affinity APIs
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = resource;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sched.getAffinity",
        ))
        .boxed())
    }
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

    // write affinity on targets that expose pthread affinity APIs
    #[cfg(target_os = "linux")]
    {
        let mut cpu_set: libc::cpu_set_t = unsafe { mem::zeroed() };
        unsafe {
            libc::CPU_ZERO(&mut cpu_set);
        }
        let cpu_set_size = std::mem::size_of::<libc::cpu_set_t>() * 8;
        for cpu in cpus {
            if cpu.group != 0 {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "cpus",
                    format!("unix thread affinity requires group 0, found {}", cpu.group),
                ))
                .boxed());
            }

            let index = cpu.cpu as usize;
            if index >= cpu_set_size {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "cpus",
                    format!("cpu index {} is out of range", cpu.cpu),
                ))
                .boxed());
            }

            unsafe {
                libc::CPU_SET(index, &mut cpu_set);
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
        let _ = (resource, cpus);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sched.setAffinity",
        ))
        .boxed())
    }

    // report unsupported affinity writes on targets without pthread affinity APIs
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (resource, cpus);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.thread.sched.setAffinity",
        ))
        .boxed())
    }
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
