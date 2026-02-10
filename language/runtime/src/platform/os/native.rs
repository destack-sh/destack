#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::os::bindings_generated as bindings;
use crate::platform::{NativeArray, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::fs;
use crate::platform::os::{HostIdentity, MountEntry, PowerState, SystemInfo};

/// Stub for destack.os.host.identity.
pub unsafe fn destack_os_host_identity(
    context: &RuntimeCallContext,
    out: *mut HostIdentity,
) -> RuntimeResult<()> {
    context.check_policy(OS_HOST_IDENTITY)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.host.identity")).boxed())
}

/// Stub for destack.os.info.systemInfo.
pub unsafe fn destack_os_system_info(
    context: &RuntimeCallContext,
    out: *mut SystemInfo,
) -> RuntimeResult<()> {
    context.check_policy(OS_INFO_SYSTEM_INFO)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.info.systemInfo")).boxed())
}

/// Stub for destack.os.mount.add.
pub unsafe fn destack_os_add(
    context: &RuntimeCallContext,
    source: fs::OsPath,
    target: fs::OsPath,
    filesystem: NativeStringRef,
    flags: u64,
    data: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(OS_MOUNT_ADD)?;
    let _ = (source, target, filesystem, flags, data);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.mount.add")).boxed())
}

/// Stub for destack.os.mount.list.
pub unsafe fn destack_os_list(
    context: &RuntimeCallContext,
    out: *mut NativeArray<MountEntry>,
) -> RuntimeResult<()> {
    context.check_policy(OS_MOUNT_LIST)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.mount.list")).boxed())
}

/// Stub for destack.os.mount.remove.
pub unsafe fn destack_os_remove(
    context: &RuntimeCallContext,
    target: fs::OsPath,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(OS_MOUNT_REMOVE)?;
    let _ = (target, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.mount.remove")).boxed())
}

/// Stub for destack.os.power.state.
pub unsafe fn destack_os_power_state(
    context: &RuntimeCallContext,
    out: *mut PowerState,
) -> RuntimeResult<()> {
    context.check_policy(OS_POWER_STATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.power.state")).boxed())
}

/// Stub for destack.os.power.suspend.
pub unsafe fn destack_os_suspend(context: &RuntimeCallContext) -> RuntimeResult<()> {
    context.check_policy(OS_POWER_SUSPEND)?;

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.power.suspend")).boxed())
}
