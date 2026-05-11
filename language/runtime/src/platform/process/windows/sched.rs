#![allow(clippy::missing_safety_doc)]
use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{
    ProcessCpuSet, ProcessId, ProcessSchedulerConfig, ProcessSchedulerPolicy, core as core_process,
};
use crate::platform::thread::ThreadCpu;
use crate::platform::{PlatformError, PlatformErrorCode, core as core_platform};
use crate::runtime::BindingCallContext;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_CALL_NOT_IMPLEMENTED, ERROR_INSUFFICIENT_BUFFER,
    ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED, FARPROC, HANDLE,
};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::System::SystemInformation::GROUP_AFFINITY;
use windows_sys::Win32::System::Threading::{
    GetPriorityClass, GetProcessAffinityMask, GetProcessGroupAffinity, OpenProcess,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, SetPriorityClass,
    SetProcessAffinityMask,
};

/// Binding operation for process-affinity reads.
const PROCESS_GET_AFFINITY_OPERATION: &str = "destack.process.sched.getAffinity";
/// Binding operation for process-affinity writes.
const PROCESS_SET_AFFINITY_OPERATION: &str = "destack.process.sched.setAffinity";
/// Kernel32 module name used for optional affinity APIs.
const KERNEL32_MODULE_NAME: &str = "kernel32.dll";
/// Optional GetProcessDefaultCpuSetMasks symbol name.
const GET_PROCESS_DEFAULT_CPU_SET_MASKS_NAME: &[u8] = b"GetProcessDefaultCpuSetMasks\0";
/// Optional SetProcessDefaultCpuSetMasks symbol name.
const SET_PROCESS_DEFAULT_CPU_SET_MASKS_NAME: &[u8] = b"SetProcessDefaultCpuSetMasks\0";

/// Native signature for GetProcessDefaultCpuSetMasks.
type GetProcessDefaultCpuSetMasksFn =
    unsafe extern "system" fn(HANDLE, *mut GROUP_AFFINITY, u16, *mut u16) -> i32;
/// Native signature for SetProcessDefaultCpuSetMasks.
type SetProcessDefaultCpuSetMasksFn =
    unsafe extern "system" fn(HANDLE, *const GROUP_AFFINITY, u16) -> i32;

/// Optional process CPU-set mask API table.
#[derive(Clone, Copy)]
struct ProcessCpuSetMaskApis {
    /// Optional query entry point.
    get_masks: Option<GetProcessDefaultCpuSetMasksFn>,
    /// Optional update entry point.
    set_masks: Option<SetProcessDefaultCpuSetMasksFn>,
}

/// RAII process-handle guard.
struct ProcessHandleGuard {
    /// Native process handle.
    handle: HANDLE,
}

impl Drop for ProcessHandleGuard {
    /// Close the guarded process handle.
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

/// Return the cached optional process CPU-set mask APIs.
fn process_cpu_set_mask_apis() -> &'static ProcessCpuSetMaskApis {
    static PROCESS_CPU_SET_MASK_APIS: OnceLock<ProcessCpuSetMaskApis> = OnceLock::new();

    PROCESS_CPU_SET_MASK_APIS.get_or_init(|| {
        let get_masks =
            resolve_kernel32_proc(GET_PROCESS_DEFAULT_CPU_SET_MASKS_NAME).map(|function| unsafe {
                std::mem::transmute::<
                    unsafe extern "system" fn() -> isize,
                    GetProcessDefaultCpuSetMasksFn,
                >(function)
            });
        let set_masks =
            resolve_kernel32_proc(SET_PROCESS_DEFAULT_CPU_SET_MASKS_NAME).map(|function| unsafe {
                std::mem::transmute::<
                    unsafe extern "system" fn() -> isize,
                    SetProcessDefaultCpuSetMasksFn,
                >(function)
            });

        ProcessCpuSetMaskApis {
            get_masks,
            set_masks,
        }
    })
}

/// Resolve one optional exported procedure from kernel32.
fn resolve_kernel32_proc(name: &[u8]) -> Option<unsafe extern "system" fn() -> isize> {
    let module_name = core_platform::wide_with_nul(KERNEL32_MODULE_NAME);
    let module = unsafe { GetModuleHandleW(module_name.as_ptr()) };
    if module == 0 {
        return None;
    }

    let function: FARPROC = unsafe { GetProcAddress(module, name.as_ptr()) };
    function
}

/// Build one process-open error for scheduler and affinity calls.
fn process_open_error(pid: u32, action: &str) -> Box<RuntimeError> {
    let error = core_platform::last_error_code() as u32;
    let code = if error == ERROR_INVALID_PARAMETER {
        PlatformErrorCode::ProcessNotFound
    } else {
        PlatformErrorCode::ProcessPermissionDenied
    };

    RuntimeError::from(PlatformError::process_with(
        Some(code),
        Some(error.to_string()),
        None,
        None,
        Some("OpenProcess".to_string()),
        format!("failed to open process {pid} for {action}"),
    ))
    .boxed()
}

/// Build one process-affinity syscall error.
fn process_affinity_error(
    operation: &str,
    syscall: &str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let error = core_platform::last_error_code() as u32;
    if matches!(error, ERROR_NOT_SUPPORTED | ERROR_CALL_NOT_IMPLEMENTED) {
        return core_platform::not_supported(operation);
    }

    let code = if error == ERROR_ACCESS_DENIED {
        PlatformErrorCode::ProcessPermissionDenied
    } else {
        PlatformErrorCode::Process
    };

    RuntimeError::from(PlatformError::process_with(
        Some(code),
        Some(error.to_string()),
        None,
        None,
        Some(syscall.to_string()),
        message,
    ))
    .boxed()
}

/// Open one process handle for affinity or scheduling work.
fn open_process_handle(
    pid: ProcessId,
    access: u32,
    action: &str,
) -> RuntimeResult<ProcessHandleGuard> {
    let pid = core_process::process_pid_to_windows_target(pid.0, "pid")?;
    let handle = unsafe { OpenProcess(access, 0, pid) };
    if handle == 0 {
        return Err(process_open_error(pid, action));
    }

    Ok(ProcessHandleGuard { handle })
}

/// Build one grouped affinity-mask vector from one runtime CPU-set payload.
fn group_affinity_masks_from_cpus(cpus: &[ThreadCpu]) -> RuntimeResult<Vec<GROUP_AFFINITY>> {
    // reject empty affinity sets eagerly
    if cpus.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "cpus",
            "process affinity set must not be empty",
        ))
        .boxed());
    }

    // collect one mask per processor group
    let mut masks_by_group = BTreeMap::<u16, usize>::new();
    for cpu in cpus {
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

        let mask = masks_by_group.entry(cpu.group).or_insert(0);
        *mask |= 1usize << bit_index;
    }

    // lower the grouped masks into Win32 structs
    let masks = masks_by_group
        .into_iter()
        .map(|(group, mask)| GROUP_AFFINITY {
            Mask: mask,
            Group: group,
            Reserved: [0, 0, 0],
        })
        .collect();

    Ok(masks)
}

/// Decode one host grouped affinity mask vector into runtime CPUs.
fn cpus_from_group_affinity_masks(masks: &[GROUP_AFFINITY]) -> RuntimeResult<Vec<ThreadCpu>> {
    let mut cpus = Vec::new();
    for mask in masks {
        for cpu in 0..usize::BITS as usize {
            let bit = 1usize << cpu;
            if (mask.Mask & bit) == 0 {
                continue;
            }

            let cpu = u16::try_from(cpu).map_err(|_| {
                RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoInvalidData),
                    None,
                    None,
                    Some("GetProcessAffinityMask".to_string()),
                    None,
                    "host cpu index exceeds process affinity ABI range",
                ))
                .boxed()
            })?;
            cpus.push(ThreadCpu {
                group: mask.Group,
                cpu,
            });
        }
    }

    Ok(cpus)
}

/// Query process groups for one target process.
fn query_process_groups(process: HANDLE, operation: &str) -> RuntimeResult<Vec<u16>> {
    // probe the group count with one minimal buffer
    let mut count = 1u16;
    let mut groups = [0u16; 1];
    let result = unsafe { GetProcessGroupAffinity(process, &mut count, groups.as_mut_ptr()) };
    if result != 0 {
        return Ok(groups[..usize::from(count)].to_vec());
    }

    // grow the group buffer when the host reports one larger group array
    let error = core_platform::last_error_code() as u32;
    if error != ERROR_INSUFFICIENT_BUFFER {
        return Err(process_affinity_error(
            operation,
            "GetProcessGroupAffinity",
            "failed to read process group affinity",
        ));
    }

    let mut groups = vec![0u16; usize::from(count)];
    let result = unsafe { GetProcessGroupAffinity(process, &mut count, groups.as_mut_ptr()) };
    if result == 0 {
        return Err(process_affinity_error(
            operation,
            "GetProcessGroupAffinity",
            "failed to read process group affinity",
        ));
    }

    groups.truncate(usize::from(count));
    Ok(groups)
}

/// Query process default CPU-set masks when the host exports that API.
fn query_process_default_cpu_set_masks(
    process: HANDLE,
    operation: &str,
) -> RuntimeResult<Option<Vec<GROUP_AFFINITY>>> {
    let Some(get_masks) = process_cpu_set_mask_apis().get_masks else {
        return Ok(None);
    };

    // probe the required mask count
    let mut required = 0u16;
    let result = unsafe { get_masks(process, std::ptr::null_mut(), 0, &mut required) };
    if result != 0 {
        return Ok(Some(Vec::new()));
    }

    // treat the expected growth path separately
    let error = core_platform::last_error_code() as u32;
    if error == ERROR_INSUFFICIENT_BUFFER {
        let mut masks = vec![
            GROUP_AFFINITY {
                Mask: 0,
                Group: 0,
                Reserved: [0, 0, 0],
            };
            usize::from(required)
        ];
        let result = unsafe { get_masks(process, masks.as_mut_ptr(), required, &mut required) };
        if result == 0 {
            return Err(process_affinity_error(
                operation,
                "GetProcessDefaultCpuSetMasks",
                "failed to read process default CPU-set masks",
            ));
        }

        masks.truncate(usize::from(required));
        return Ok(Some(masks));
    }

    // preserve older-host fallback behavior
    if matches!(error, ERROR_NOT_SUPPORTED | ERROR_CALL_NOT_IMPLEMENTED) {
        return Ok(None);
    }

    Err(process_affinity_error(
        operation,
        "GetProcessDefaultCpuSetMasks",
        "failed to read process default CPU-set masks",
    ))
}

/// Apply process default CPU-set masks when the host exports that API.
fn set_process_default_cpu_set_masks(
    process: HANDLE,
    masks: &[GROUP_AFFINITY],
    operation: &str,
) -> RuntimeResult<bool> {
    let Some(set_masks) = process_cpu_set_mask_apis().set_masks else {
        return Ok(false);
    };

    let mask_count = u16::try_from(masks.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "cpus",
            "processor-group set exceeds Windows CPU-set API range",
        ))
        .boxed()
    })?;

    let result = unsafe { set_masks(process, masks.as_ptr(), mask_count) };
    if result == 0 {
        return Err(process_affinity_error(
            operation,
            "SetProcessDefaultCpuSetMasks",
            "failed to set process default CPU-set masks",
        ));
    }

    Ok(true)
}

/// Query one legacy process affinity mask from older Windows hosts.
fn query_legacy_process_affinity(
    process: HANDLE,
    operation: &str,
) -> RuntimeResult<Vec<GROUP_AFFINITY>> {
    // require one legacy single-group process target
    let groups = query_process_groups(process, operation)?;
    if groups.len() != 1 {
        return Err(core_platform::not_supported(operation));
    }

    // read the process affinity mask from the legacy API
    let mut process_mask = 0usize;
    let mut system_mask = 0usize;
    let result = unsafe { GetProcessAffinityMask(process, &mut process_mask, &mut system_mask) };
    if result == 0 {
        return Err(process_affinity_error(
            operation,
            "GetProcessAffinityMask",
            "failed to read process affinity mask",
        ));
    }

    Ok(vec![GROUP_AFFINITY {
        Mask: process_mask,
        Group: groups[0],
        Reserved: [0, 0, 0],
    }])
}

/// Apply one legacy process affinity mask on older Windows hosts.
fn set_legacy_process_affinity(
    process: HANDLE,
    masks: &[GROUP_AFFINITY],
    operation: &str,
) -> RuntimeResult<()> {
    // reject multi-group requests on the legacy process-affinity API
    if masks.len() != 1 {
        return Err(core_platform::not_supported(operation));
    }

    // require one single-group target process for the legacy API
    let groups = query_process_groups(process, operation)?;
    if groups.len() != 1 || groups[0] != masks[0].Group {
        return Err(core_platform::not_supported(operation));
    }

    // apply the legacy process affinity mask
    let result = unsafe { SetProcessAffinityMask(process, masks[0].Mask) };
    if result == 0 {
        return Err(process_affinity_error(
            operation,
            "SetProcessAffinityMask",
            "failed to set process affinity mask",
        ));
    }

    Ok(())
}
/// Read process CPU affinity.
pub(crate) unsafe fn destack_process_get_affinity(
    binding: &BindingCallContext,
    out: *mut ProcessCpuSet,
    pid: ProcessId,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the target process for affinity reads
    let process = open_process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION, "affinity read")?;

    // prefer CPU-set masks when the host exports that API
    let masks = match query_process_default_cpu_set_masks(
        process.handle,
        PROCESS_GET_AFFINITY_OPERATION,
    )? {
        Some(masks) if !masks.is_empty() => masks,
        _ => query_legacy_process_affinity(process.handle, PROCESS_GET_AFFINITY_OPERATION)?,
    };

    // lower the host masks into one runtime CPU-set payload
    let cpus = cpus_from_group_affinity_masks(&masks)?;
    let value = ProcessCpuSet {
        cpus: binding.store_array(cpus),
    };
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read a process priority value.
pub(crate) unsafe fn destack_process_get_priority(
    _binding: &BindingCallContext,
    out: *mut i32,
    pid: ProcessId,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the target process for priority reads
    let process = open_process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION, "priority read")?;

    let class = unsafe { GetPriorityClass(process.handle) };
    let value = if class == 0 {
        let error = core_platform::last_error_code();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read process priority class: {error}",
        )))
        .boxed());
    } else {
        windows_priority_class_to_nice(class)?
    };

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

    // open the target process for scheduler-class reads
    let process = open_process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION, "scheduler read")?;

    let class = unsafe { GetPriorityClass(process.handle) };
    if class == 0 {
        let error = core_platform::last_error_code();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read process scheduler class: {error}",
        )))
        .boxed());
    }

    let config = scheduler_config_from_priority_class(class)?;
    unsafe {
        *out = config;
    }

    Ok(())
}

/// Set process CPU affinity.
pub(crate) unsafe fn destack_process_set_affinity(
    _binding: &BindingCallContext,
    pid: ProcessId,
    cpus: ProcessCpuSet,
) -> RuntimeResult<()> {
    let cpus = unsafe { cpus.cpus.as_slice()? };

    // lower the requested CPU set into grouped masks
    let masks = group_affinity_masks_from_cpus(cpus)?;

    // open the target process for affinity updates
    let process = open_process_handle(
        pid,
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_INFORMATION,
        "affinity update",
    )?;

    // prefer CPU-set masks when the host exports that API
    if set_process_default_cpu_set_masks(process.handle, &masks, PROCESS_SET_AFFINITY_OPERATION)? {
        return Ok(());
    }

    // otherwise use the honest legacy single-group fallback
    set_legacy_process_affinity(process.handle, &masks, PROCESS_SET_AFFINITY_OPERATION)
}

/// Set a process priority value.
pub(crate) unsafe fn destack_process_set_priority(
    _binding: &BindingCallContext,
    pid: ProcessId,
    priority: i32,
) -> RuntimeResult<()> {
    // open the target process for priority updates
    let process = open_process_handle(pid, PROCESS_SET_INFORMATION, "priority update")?;

    let priority_class = windows_nice_to_priority_class(priority);
    let result = unsafe { SetPriorityClass(process.handle, priority_class) };
    if result == 0 {
        let error = core_platform::last_error_code();
        Err(RuntimeError::from(PlatformError::io(format!(
            "failed to set process priority class: {error}",
        )))
        .boxed())
    } else {
        Ok(())
    }
}

/// Set scheduler policy and priority for a process.
pub(crate) unsafe fn destack_process_set_scheduler(
    _binding: &BindingCallContext,
    pid: ProcessId,
    config: ProcessSchedulerConfig,
) -> RuntimeResult<()> {
    // reject unsupported scheduler flags eagerly
    if config.flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "config.flags",
            "windows process scheduler flags are not supported",
        ))
        .boxed());
    }

    // open the target process for scheduler-class updates
    let process = open_process_handle(pid, PROCESS_SET_INFORMATION, "scheduler update")?;

    let priority_class = priority_class_from_scheduler_config(config)?;
    let result = unsafe { SetPriorityClass(process.handle, priority_class) };
    if result == 0 {
        let error = core_platform::last_error_code();
        Err(RuntimeError::from(PlatformError::io(format!(
            "failed to set process scheduler class: {error}",
        )))
        .boxed())
    } else {
        Ok(())
    }
}

/// Yield the current thread to the scheduler.
pub(crate) unsafe fn destack_process_yield_now(_binding: &BindingCallContext) -> RuntimeResult<()> {
    use windows_sys::Win32::System::Threading::{Sleep, SwitchToThread};

    let switched = unsafe { SwitchToThread() };
    if switched == 0 {
        unsafe { Sleep(0) };
    }

    Ok(())
}

/// Map a Windows process priority class to a unix-style nice value.
fn windows_priority_class_to_nice(class: u32) -> RuntimeResult<i32> {
    use windows_sys::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    };

    if class == REALTIME_PRIORITY_CLASS {
        return Ok(-20);
    }
    if class == HIGH_PRIORITY_CLASS {
        return Ok(-10);
    }
    if class == ABOVE_NORMAL_PRIORITY_CLASS {
        return Ok(-5);
    }
    if class == NORMAL_PRIORITY_CLASS {
        return Ok(0);
    }
    if class == BELOW_NORMAL_PRIORITY_CLASS {
        return Ok(10);
    }
    if class == IDLE_PRIORITY_CLASS {
        return Ok(19);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "priorityClass",
        format!("unsupported priority class {class}"),
    ))
    .boxed())
}

/// Map a unix-style nice value to a Windows process priority class.
fn windows_nice_to_priority_class(priority: i32) -> u32 {
    use windows_sys::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    };

    if priority <= -15 {
        return REALTIME_PRIORITY_CLASS;
    }
    if priority <= -8 {
        return HIGH_PRIORITY_CLASS;
    }
    if priority <= -3 {
        return ABOVE_NORMAL_PRIORITY_CLASS;
    }
    if priority <= 4 {
        return NORMAL_PRIORITY_CLASS;
    }
    if priority <= 10 {
        return BELOW_NORMAL_PRIORITY_CLASS;
    }

    IDLE_PRIORITY_CLASS
}

/// Map a Windows priority class into one scheduler configuration.
fn scheduler_config_from_priority_class(class: u32) -> RuntimeResult<ProcessSchedulerConfig> {
    use windows_sys::Win32::System::Threading::{BELOW_NORMAL_PRIORITY_CLASS, IDLE_PRIORITY_CLASS};

    let priority = windows_priority_class_to_nice(class)?;
    let policy = if class == IDLE_PRIORITY_CLASS {
        ProcessSchedulerPolicy::Idle
    } else if class == BELOW_NORMAL_PRIORITY_CLASS {
        ProcessSchedulerPolicy::Batch
    } else {
        ProcessSchedulerPolicy::Other
    };

    Ok(ProcessSchedulerConfig {
        policy,
        priority,
        flags: 0,
    })
}

/// Map one scheduler configuration into a Windows priority class.
fn priority_class_from_scheduler_config(config: ProcessSchedulerConfig) -> RuntimeResult<u32> {
    use windows_sys::Win32::System::Threading::{BELOW_NORMAL_PRIORITY_CLASS, IDLE_PRIORITY_CLASS};

    if config.policy == ProcessSchedulerPolicy::Idle {
        return Ok(IDLE_PRIORITY_CLASS);
    }
    if config.policy == ProcessSchedulerPolicy::Batch {
        return Ok(BELOW_NORMAL_PRIORITY_CLASS);
    }
    if config.policy == ProcessSchedulerPolicy::Other {
        return Ok(windows_nice_to_priority_class(config.priority));
    }

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setScheduler",
    ))
    .boxed())
}
