#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::process::core as core_process;
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::thread::ThreadCpu;

use crate::runtime::BindingCallContext;

#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::process::ProcessSchedulerPolicy;
use crate::platform::process::{ProcessCpuSet, ProcessId, ProcessSchedulerConfig};

/// Read process CPU affinity.
pub(crate) unsafe fn destack_process_get_affinity(
    binding: &BindingCallContext,
    out: *mut ProcessCpuSet,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let pid = core_process::process_pid_to_unix_target(pid.0, "pid")?;

        let mut cpu_set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        unsafe {
            libc::CPU_ZERO(&mut cpu_set);
        }

        let result = unsafe {
            libc::sched_getaffinity(pid, std::mem::size_of::<libc::cpu_set_t>(), &mut cpu_set)
        };
        if result != 0 {
            return Err(core_process::process_last_error(
                "sched_getaffinity",
                format!("failed to read affinity for pid {}", pid as u32),
            ));
        }

        let cpu_set_size = std::mem::size_of::<libc::cpu_set_t>() * 8;
        let mut cpus = Vec::new();
        for cpu in 0..cpu_set_size {
            let is_member = unsafe { libc::CPU_ISSET(cpu, &cpu_set) };
            if is_member {
                let cpu = u16::try_from(cpu).map_err(|_| {
                    RuntimeError::from(PlatformError::io_with(
                        None,
                        None,
                        None,
                        Some("sched_getaffinity".to_string()),
                        None,
                        "host cpu index exceeds process affinity ABI range",
                    ))
                    .boxed()
                })?;
                cpus.push(ThreadCpu { group: 0, cpu });
            }
        }

        let value = ProcessCpuSet {
            cpus: binding.store_array(cpus),
        };
        unsafe {
            *out = value;
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (binding, pid);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.getAffinity",
        ))
        .boxed())
    }
}

/// Read a process priority value.
pub(crate) unsafe fn destack_process_get_priority(
    _binding: &BindingCallContext,
    out: *mut i32,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let pid = core_process::process_pid_to_unix_target(pid.0, "pid")? as libc::id_t;

    unsafe {
        *errno_location() = 0;
    }

    let value = unsafe { libc::getpriority(libc::PRIO_PROCESS, pid) };
    if value == -1 {
        let errno = unsafe { *errno_location() };
        if errno != 0 {
            return Err(core_process::process_errno_error(
                errno,
                "getpriority",
                format!("failed to get priority for pid {}", pid as u32),
            ));
        }
    }

    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read scheduler policy and priority for a process.
pub(crate) unsafe fn destack_process_get_scheduler(
    _binding: &BindingCallContext,
    out: *mut ProcessSchedulerConfig,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let pid = core_process::process_pid_to_unix_target(pid.0, "pid")?;
        let policy = unsafe { libc::sched_getscheduler(pid) };
        if policy < 0 {
            return Err(core_process::process_last_error(
                "sched_getscheduler",
                format!("failed to read scheduler policy for pid {}", pid as u32),
            ));
        }

        let mut raw_param = unsafe { std::mem::zeroed::<libc::sched_param>() };
        let param_result = unsafe { libc::sched_getparam(pid, &mut raw_param) };
        if param_result != 0 {
            return Err(core_process::process_last_error(
                "sched_getparam",
                format!("failed to read scheduler priority for pid {}", pid as u32),
            ));
        }

        let policy = scheduler_policy_from_raw(policy)?;
        let value = ProcessSchedulerConfig {
            policy,
            priority: raw_param.sched_priority,
            flags: 0,
        };
        unsafe {
            *out = value;
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = pid;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.getScheduler",
        ))
        .boxed())
    }
}

/// Set process CPU affinity.
pub(crate) unsafe fn destack_process_set_affinity(
    _binding: &BindingCallContext,
    pid: ProcessId,
    cpus: ProcessCpuSet,
) -> RuntimeResult<()> {
    let cpus = unsafe { cpus.cpus.as_slice()? };
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let pid = core_process::process_pid_to_unix_target(pid.0, "pid")?;

        let mut cpu_set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        unsafe {
            libc::CPU_ZERO(&mut cpu_set);
        }

        let cpu_set_size = std::mem::size_of::<libc::cpu_set_t>() * 8;
        for cpu in cpus {
            if cpu.group != 0 {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "cpus",
                    format!(
                        "unix process affinity requires group 0, found {}",
                        cpu.group
                    ),
                ))
                .boxed());
            }

            let index = usize::from(cpu.cpu);
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

        let result = unsafe {
            libc::sched_setaffinity(pid, std::mem::size_of::<libc::cpu_set_t>(), &cpu_set)
        };
        if result != 0 {
            return Err(core_process::process_last_error(
                "sched_setaffinity",
                format!("failed to set affinity for pid {}", pid as u32),
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (pid, cpus);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.setAffinity",
        ))
        .boxed())
    }
}

/// Set scheduler policy and priority for a process.
pub(crate) unsafe fn destack_process_set_scheduler(
    _binding: &BindingCallContext,
    pid: ProcessId,
    config: ProcessSchedulerConfig,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let pid = core_process::process_pid_to_unix_target(pid.0, "pid")?;
        if config.flags != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "config.flags",
                "scheduler flags are not supported yet",
            ))
            .boxed());
        }

        let policy = scheduler_policy_to_raw(config.policy)?;
        let mut raw_param = unsafe { std::mem::zeroed::<libc::sched_param>() };
        raw_param.sched_priority = config.priority;

        let result = unsafe { libc::sched_setscheduler(pid, policy, &raw_param) };
        if result != 0 {
            return Err(core_process::process_last_error(
                "sched_setscheduler",
                format!("failed to set scheduler for pid {}", pid as u32),
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (pid, config);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.setScheduler",
        ))
        .boxed())
    }
}

/// Set a process priority value.
pub(crate) unsafe fn destack_process_set_priority(
    _binding: &BindingCallContext,
    pid: ProcessId,
    priority: i32,
) -> RuntimeResult<()> {
    let pid = core_process::process_pid_to_unix_target(pid.0, "pid")? as libc::id_t;
    let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid, priority) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "setpriority",
            format!("failed to set priority for pid {}", pid as u32),
        ));
    }

    Ok(())
}

/// Yield the current thread to the scheduler.
pub(crate) unsafe fn destack_process_yield_now(_binding: &BindingCallContext) -> RuntimeResult<()> {
    let result = unsafe { libc::sched_yield() };
    if result != 0 {
        return Err(core_process::process_last_error(
            "sched_yield",
            "failed to yield scheduler timeslice",
        ));
    }

    Ok(())
}

/// Convert a scheduler policy enum into a host scheduler constant.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn scheduler_policy_to_raw(policy: ProcessSchedulerPolicy) -> RuntimeResult<libc::c_int> {
    match policy {
        ProcessSchedulerPolicy::Other => Ok(scheduler_other_policy()),
        ProcessSchedulerPolicy::Fifo => Ok(libc::SCHED_FIFO),
        ProcessSchedulerPolicy::RoundRobin => Ok(libc::SCHED_RR),
        ProcessSchedulerPolicy::Batch => Ok(libc::SCHED_BATCH),
        ProcessSchedulerPolicy::Idle => Ok(libc::SCHED_IDLE),
        ProcessSchedulerPolicy::Deadline => Ok(libc::SCHED_DEADLINE),
    }
}

/// Convert a host scheduler constant into a scheduler policy enum.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn scheduler_policy_from_raw(policy: libc::c_int) -> RuntimeResult<ProcessSchedulerPolicy> {
    if policy == scheduler_other_policy() {
        return Ok(ProcessSchedulerPolicy::Other);
    }
    if policy == libc::SCHED_FIFO {
        return Ok(ProcessSchedulerPolicy::Fifo);
    }
    if policy == libc::SCHED_RR {
        return Ok(ProcessSchedulerPolicy::RoundRobin);
    }
    if policy == libc::SCHED_BATCH {
        return Ok(ProcessSchedulerPolicy::Batch);
    }
    if policy == libc::SCHED_IDLE {
        return Ok(ProcessSchedulerPolicy::Idle);
    }
    if policy == libc::SCHED_DEADLINE {
        return Ok(ProcessSchedulerPolicy::Deadline);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "policy",
        format!("unsupported scheduler policy value {policy}"),
    ))
    .boxed())
}

/// Return the host constant for the standard scheduler policy.
#[cfg(target_os = "android")]
fn scheduler_other_policy() -> libc::c_int {
    libc::SCHED_NORMAL
}

/// Return the host constant for the standard scheduler policy.
#[cfg(target_os = "linux")]
fn scheduler_other_policy() -> libc::c_int {
    libc::SCHED_OTHER
}

/// Return a mutable pointer to the host errno slot.
#[cfg(all(unix, target_os = "linux"))]
unsafe fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

/// Return a mutable pointer to the host errno slot.
#[cfg(all(unix, target_os = "android"))]
unsafe fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno() }
}

/// Return a mutable pointer to the host errno slot.
#[cfg(all(
    unix,
    any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
unsafe fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}
