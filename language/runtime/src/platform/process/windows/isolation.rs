#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::fs::core as core_fs;
use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions, ProcessStdio,
    ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};
/// Change the root directory for path resolution.
///
/// Replace process root path resolution context with the provided directory.
/// Root-change semantics are host-defined and privilege-gated.
///
/// # Platform
/// Unix.
/// Uses chroot(2) or equivalent jail primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `security.restrict`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_chroot(
    _context: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let _path = core_fs::os_path_to_utf8_string(path, "path")?;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.chroot",
    ))
    .boxed())
}

/// Install one syscall filter program.
///
/// Install one host syscall filter for the current process.
/// Program bytecode and verifier rules are host-specific.
///
/// # Platform
/// Linux.
/// Uses seccomp filter install primitives.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `security.filter`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_install_syscall_filter(
    _context: &BindingCallContext,
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
///
/// Update host name for the current UTS namespace or host context.
/// Name-length and privilege rules are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses sethostname(2) on Unix and host name APIs on Windows where permitted.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.namespace`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_set_host_name(
    _context: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _name = unsafe { name.as_str()? };
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setHostName",
    ))
    .boxed())
}

/// Set network namespace context for subsequent network operations.
///
/// Switch network operation context to the specified network namespace path.
/// Namespace transition rules and privileges are host-defined.
///
/// # Platform
/// Linux.
/// Uses setns-style namespace switching with network namespace descriptors.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.namespace`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_set_network_namespace(
    _context: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let _path = core_fs::os_path_to_utf8_string(path, "path")?;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setNetworkNamespace",
    ))
    .boxed())
}

/// Enter one namespace owned by another process.
///
/// Join one specific namespace type from the target process.
/// Namespace join rules and privilege checks are enforced by the host kernel.
///
/// # Platform
/// Linux.
/// Uses setns(2) with namespace file descriptors.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.namespace`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_setns(
    _context: &BindingCallContext,
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
///
/// Create isolated namespaces for the current process according to flag bits.
/// Namespace semantics and inheritance follow host kernel rules.
///
/// # Platform
/// Linux.
/// Uses unshare(2).
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.namespace`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_unshare(
    _context: &BindingCallContext,
    _flags: ProcessUnshareFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.unshare",
    ))
    .boxed())
}
