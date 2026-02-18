#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::os::{HostIdentityVm, LoadAverageVm, MountEntryVm, PowerState, SystemInfoVm};
use crate::platform::{PlatformError, VmArray, fs};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Read host identity.
///
/// Return one normalized host identity payload.
/// Identity fields are sourced from host kernel and runtime normalization rules.
///
/// # Platform
/// Unix and Windows.
/// Uses uname and hostname APIs on Unix and version and hostname APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.hostname`, `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_host_identity(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<HostIdentityVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.host.identity is not available in the VM yet",
    ))
    .boxed())
}

/// Read host boot time.
///
/// Return the Unix timestamp for host boot time in nanoseconds.
/// Timestamp origin and precision follow host timekeeping interfaces.
///
/// # Platform
/// Unix and Windows.
/// Uses boot-time sysctl or procfs style sources on Unix and boot-time system info on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_boot_time_unix_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.info.bootTimeUnixNs is not available in the VM yet",
    ))
    .boxed())
}

/// Read host load averages.
///
/// Return host load averages over one, five, and fifteen minute windows.
/// Values reflect host scheduler accounting and may be unavailable on some kernels.
///
/// # Platform
/// Unix only.
/// Uses getloadavg style interfaces or kernel load-average exports.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_load_average(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<LoadAverageVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.info.loadAverage is not available in the VM yet",
    ))
    .boxed())
}

/// Read host system information.
///
/// Return one normalized system-information payload.
/// Topology and capacity fields are sampled from host APIs at call time.
///
/// # Platform
/// Unix and Windows.
/// Uses sysconf/sysinfo-style APIs on Unix and GlobalMemoryStatusEx plus processor APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_system_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<SystemInfoVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.info.systemInfo is not available in the VM yet",
    ))
    .boxed())
}

/// Read host uptime.
///
/// Return host uptime in nanoseconds from system boot.
/// Uptime source follows host monotonic uptime facilities.
///
/// # Platform
/// Unix and Windows.
/// Uses clock_gettime style uptime on Unix and GetTickCount64 style uptime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_uptime_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.info.uptimeNs is not available in the VM yet",
    ))
    .boxed())
}

/// Mount one filesystem target.
///
/// Mount one source on one target path with explicit flags and data.
/// Mount privilege checks and propagation policy are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses mount(2)-family APIs on Unix and volume-mount APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.mount`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_add(
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

/// Enumerate mount table entries.
///
/// Return one snapshot of the current host mount table.
/// Entry shape is normalized but field availability is host-dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses mount table APIs on Unix and volume enumeration APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.mount`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<MountEntryVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.mount.list is not available in the VM yet",
    ))
    .boxed())
}

/// Unmount one filesystem target.
///
/// Unmount one target path with explicit unmount flags.
/// Forced unmount behavior follows host kernel semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses umount or unmount APIs on Unix and volume unmount APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.mount`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_remove(
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

/// Read current host power state.
///
/// Return one normalized host power-state classification.
/// State mapping follows runtime normalization over host power APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses host power management APIs such as sysfs and IOKit on Unix-like systems and GetSystemPowerStatus on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.power`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_power_state(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<PowerState> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.power.state is not available in the VM yet",
    ))
    .boxed())
}

/// Request host suspend.
///
/// Request one host suspend transition through platform power APIs.
/// Request acceptance and timing are host-policy and privilege dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses host power-management APIs where supported.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.power`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_os_suspend(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.power.suspend is not available in the VM yet",
    ))
    .boxed())
}
