#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::process::core as core_process;

use crate::runtime::BindingCallContext;

use crate::platform::process::{ProcessLimit, ProcessLimitResource};

/// Read a process resource limit.
pub(crate) unsafe fn destack_process_get_limit(
    _binding: &BindingCallContext,
    out: *mut ProcessLimit,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let mut raw_limit = unsafe { std::mem::zeroed::<libc::rlimit>() };
    let result = unsafe { libc::getrlimit(resource.0 as _, &mut raw_limit) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "getrlimit",
            format!("failed to get limit for resource {}", resource.0),
        ));
    }

    let value = ProcessLimit {
        soft: raw_limit.rlim_cur,
        hard: raw_limit.rlim_max,
    };
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Set a process resource limit.
pub(crate) unsafe fn destack_process_set_limit(
    _binding: &BindingCallContext,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    if limit.soft > limit.hard {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "limit.soft",
            "soft limit must be less than or equal to hard limit",
        ))
        .boxed());
    }

    let raw_limit = libc::rlimit {
        rlim_cur: limit.soft as libc::rlim_t,
        rlim_max: limit.hard as libc::rlim_t,
    };

    let result = unsafe { libc::setrlimit(resource.0 as _, &raw_limit) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "setrlimit",
            format!("failed to set limit for resource {}", resource.0),
        ));
    }

    Ok(())
}
