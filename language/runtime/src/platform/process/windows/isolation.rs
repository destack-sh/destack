#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::fs;
use crate::platform::fs::core as core_fs;
use crate::platform::process::{
    ProcessId, ProcessNamespaceKind, ProcessUnshareFlags, SyscallFilterFlags,
};
/// Change the root directory for path resolution.
pub(crate) unsafe fn destack_process_chroot(
    _binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let _path = core_fs::os_path_to_utf8_string(path, "path")?;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.chroot",
    ))
    .boxed())
}

/// Install one syscall filter program.
pub(crate) unsafe fn destack_process_install_syscall_filter(
    _binding: &BindingCallContext,
    program: NativeArray<u8>,
    flags: SyscallFilterFlags,
) -> RuntimeResult<()> {
    let program = unsafe { program.as_slice()? };
    let _ = (program, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.installSyscallFilter",
    ))
    .boxed())
}

/// Set process host name inside the active UTS namespace.
pub(crate) unsafe fn destack_process_set_host_name(
    _binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _name = unsafe { name.as_str()? };
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setHostName",
    ))
    .boxed())
}

/// Set network namespace context for subsequent network operations.
pub(crate) unsafe fn destack_process_set_network_namespace(
    _binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let _path = core_fs::os_path_to_utf8_string(path, "path")?;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setNetworkNamespace",
    ))
    .boxed())
}

/// Enter one namespace owned by another process.
pub(crate) unsafe fn destack_process_setns(
    _binding: &BindingCallContext,
    pid: ProcessId,
    namespace: ProcessNamespaceKind,
) -> RuntimeResult<()> {
    let _ = (pid, namespace);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setns",
    ))
    .boxed())
}

/// Unshare one or more namespaces.
pub(crate) unsafe fn destack_process_unshare(
    _binding: &BindingCallContext,
    _flags: ProcessUnshareFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.unshare",
    ))
    .boxed())
}
