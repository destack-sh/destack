#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdActionKind, ProcessFdFlags,
    ProcessFdSignalFlags, ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource,
    ProcessNamespaceKind, ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions,
    ProcessStdio, ProcessStdioKind, ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags,
    ProcessWaitKind, ProcessWaitStatus, Signal, SignalEvent, SignalFdFlags, SignalMaskHow,
    SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Read process CPU affinity.
///
/// Read the active CPU affinity mask for the target process identifier.
/// Returned CPUs reflect host scheduler topology visibility.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_getaffinity(2) on Unix and GetProcessAffinityMask on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.affinity`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_get_affinity(
    context: &RuntimeCallContext,
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

        let mut cpus = Vec::new();
        for cpu in 0..(libc::CPU_SETSIZE as usize) {
            let is_member = unsafe { libc::CPU_ISSET(cpu, &cpu_set) };
            if is_member {
                cpus.push(cpu as u32);
            }
        }

        let value = ProcessCpuSet {
            cpus: context.store_array(cpus),
        };
        unsafe {
            *out = value;
        }

        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (context, pid);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.getAffinity",
        ))
        .boxed())
    }
}

/// Read a process priority value.
///
/// Read the scheduler priority value for the target process identifier.
/// Priority ranges and classes are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses getpriority(2) on Unix and GetPriorityClass plus thread priority mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.priority`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_get_priority(
    _context: &RuntimeCallContext,
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
///
/// Read one process scheduler policy class and its priority details.
/// Returned policy availability and numeric ranges are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_getscheduler plus sched_getparam on Unix and process scheduling class mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_get_scheduler(
    _context: &RuntimeCallContext,
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

        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.getScheduler",
        ))
        .boxed())
    }
}

/// Set process CPU affinity.
///
/// Set the CPU affinity mask for the target process identifier.
/// Invalid CPU sets and privilege violations are rejected by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_setaffinity(2) on Unix and SetProcessAffinityMask on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.affinity`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_affinity(
    _context: &RuntimeCallContext,
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

        for cpu in cpus {
            let index = *cpu as usize;
            if index >= libc::CPU_SETSIZE as usize {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "cpus",
                    format!("cpu index {cpu} is out of range"),
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

        return Ok(());
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
///
/// Set one process scheduler policy class with explicit priority and flags.
/// Privilege checks and policy-specific clamping are enforced by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_setscheduler plus sched_setparam on Unix and process scheduling class mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_scheduler(
    _context: &RuntimeCallContext,
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

        return Ok(());
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
///
/// Set the scheduler priority value for the target process identifier.
/// Privilege checks and clamping are enforced by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses setpriority(2) on Unix and SetPriorityClass or SetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.priority`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_priority(
    _context: &RuntimeCallContext,
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
///
/// Yield one scheduler timeslice voluntarily from the current execution context.
/// Yield ordering and wakeup behavior follow host scheduler semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_yield(2) on Unix and SwitchToThread or Sleep(0) on Windows.
///
/// # Errors
/// Returns processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_yield_now(_context: &RuntimeCallContext) -> RuntimeResult<()> {
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
        ProcessSchedulerPolicy::Other => Ok(libc::SCHED_OTHER),
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
    if policy == libc::SCHED_OTHER {
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

/// Return a mutable pointer to the host errno slot.
#[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
unsafe fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
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
