#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::{NativeSlice, NativeStringRef};

use crate::runtime::BindingCallContext;

use crate::platform::process::{ProcessId, ProcessLimit, ProcessLimitResource};

/// Read one control-group resource limit.
pub(crate) unsafe fn destack_process_cgroup_get_limit(
    _binding: &BindingCallContext,
    out: *mut ProcessLimit,
    path: NativeStringRef,
    _resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = unsafe { path.as_str()? };
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupGetLimit",
    ))
    .boxed())
}

/// Join one control group.
pub(crate) unsafe fn destack_process_cgroup_join(
    _binding: &BindingCallContext,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = unsafe { path.as_str()? };

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupJoin",
    ))
    .boxed())
}

/// Write one control-group resource limit.
pub(crate) unsafe fn destack_process_cgroup_set_limit(
    _binding: &BindingCallContext,
    path: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = unsafe { path.as_str()? };
    let _ = (resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupSetLimit",
    ))
    .boxed())
}

/// Assign processes to one Windows job object.
pub(crate) unsafe fn destack_process_job_assign(
    _binding: &BindingCallContext,
    name: NativeStringRef,
    pids: NativeSlice<ProcessId>,
) -> RuntimeResult<()> {
    let _ = unsafe { name.as_str()? };
    let _ = unsafe { pids.as_slice()? };

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobAssign",
    ))
    .boxed())
}

/// Set one Windows job object resource limit.
pub(crate) unsafe fn destack_process_job_set_limit(
    _binding: &BindingCallContext,
    name: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = unsafe { name.as_str()? };
    let _ = (resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobSetLimit",
    ))
    .boxed())
}
