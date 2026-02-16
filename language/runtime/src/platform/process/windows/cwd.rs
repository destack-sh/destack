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

use crate::platform::fs::core as core_fs;
use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdActionKind, ProcessFdFlags,
    ProcessFdSignalFlags, ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource,
    ProcessNamespaceKind, ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions,
    ProcessStdio, ProcessStdioKind, ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags,
    ProcessWaitKind, ProcessWaitStatus, Signal, SignalEvent, SignalFdFlags, SignalMaskHow,
    SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};
/// Change the current working directory.
///
/// Update process working directory state for subsequent relative path resolution.
/// Directory existence and permission checks are performed by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses chdir(2) on Unix and SetCurrentDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.workdir.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_chdir(
    context: &RuntimeCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_CWD_CHDIR)?;
    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    core_process::process_chdir(&path)
}

/// Return the current working directory.
///
/// Query the calling process working directory without changing process state.
/// The returned path preserves host encoding through `OsPath`.
///
/// # Platform
/// Unix and Windows.
/// Uses getcwd(3) on Unix and GetCurrentDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.workdir.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_cwd(
    context: &RuntimeCallContext,
    out: *mut fs::OsPath,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_CWD_CWD)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let cwd = core_process::process_cwd()?;
    let path = core_fs::os_path_from_utf8_string(context, cwd);

    unsafe {
        *out = path;
    }

    Ok(())
}
