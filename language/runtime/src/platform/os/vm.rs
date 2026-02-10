use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::os::{HostIdentityVm, MountEntryVm, PowerState, SystemInfoVm};
use crate::platform::{PlatformError, VmArray, fs};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.os.host.identity.
pub(super) fn destack_os_host_identity(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<HostIdentityVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.host.identity is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.os.info.systemInfo.
pub(super) fn destack_os_system_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<SystemInfoVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.info.systemInfo is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.os.mount.add.
pub(super) fn destack_os_add(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    source: fs::OsPathVm,
    target: fs::OsPathVm,
    filesystem: vm::StringHandle,
    flags: u64,
    data: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (source, target, filesystem, flags, data);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.mount.add is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.os.mount.list.
pub(super) fn destack_os_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<MountEntryVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.mount.list is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.os.mount.remove.
pub(super) fn destack_os_remove(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    target: fs::OsPathVm,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (target, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.mount.remove is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.os.power.state.
pub(super) fn destack_os_power_state(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<PowerState> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.power.state is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.os.power.suspend.
pub(super) fn destack_os_suspend(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.power.suspend is not available in the VM yet",
    ))
    .boxed())
}
