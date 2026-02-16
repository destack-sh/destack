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

/// Return the effective group identifier.
///
/// Read the effective primary group for the calling process.
/// Effective group can differ from the real group under setgid-style execution.
///
/// # Platform
/// Unix and Windows.
/// Uses getegid(2) on Unix and token group translation on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_egid(
    context: &RuntimeCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = GroupId(not_supported("destack.process.egid")?);
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return the effective user identifier.
///
/// Read the effective user identity for the calling process.
/// Effective identity can differ from the real identity under setuid-style execution.
///
/// # Platform
/// Unix and Windows.
/// Uses geteuid(2) on Unix and token SID translation on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_euid(
    context: &RuntimeCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = UserId(not_supported("destack.process.euid")?);
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return the current group identifier.
///
/// Read the real primary group for the calling process.
/// Group mapping on Windows is best-effort and may require token translation.
///
/// # Platform
/// Unix and Windows.
/// Uses getgid(2) on Unix and token group translation on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_gid(
    context: &RuntimeCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = GroupId(not_supported("destack.process.gid")?);
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return real, effective, and saved-set group identifiers.
///
/// Read all group-id classes for the calling process in one operation.
/// Saved-id availability follows host kernel semantics and can be unsupported on some targets.
///
/// # Platform
/// Unix and Windows.
/// Uses getresgid(2) on Unix and token mapping or `notSupported` on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_group_ids(
    context: &RuntimeCallContext,
    out: *mut ProcessGroupIds,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = not_supported("destack.process.groupIds")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return supplementary group identifiers.
///
/// Read the supplementary group list attached to the calling process.
/// Group ordering follows host kernel reporting order.
///
/// # Platform
/// Unix and Windows.
/// Uses getgroups(2) on Unix and token group enumeration on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_groups(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<GroupId>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let groups = not_supported("destack.process.groups")?;
    unsafe {
        *out = context.store_slice(groups);
    }

    Ok(())
}

/// Return the current process identifier.
///
/// Read the caller process id from the host process table.
/// The value is stable for the lifetime of the process.
///
/// # Platform
/// Unix and Windows.
/// Uses getpid(2) on Unix and GetCurrentProcessId on Windows.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_pid(
    context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = ProcessId(unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() });
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return the parent process identifier.
///
/// Read the parent id relation as reported by the host kernel.
/// Parent visibility on Windows can be restricted by host policy.
///
/// # Platform
/// Unix and Windows.
/// Uses getppid(2) on Unix and process snapshot APIs on Windows.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_ppid(
    context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = ProcessId(not_supported("destack.process.ppid")?);
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Set the effective group identifier only.
///
/// Change only the effective primary group while preserving the real group.
/// Privilege checks and saved-ID rules are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses setegid(2) on Unix and token adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_egid(
    context: &RuntimeCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = (context, groupid);
    not_supported("destack.process.ids.setEgid")
}

/// Set the effective user identifier only.
///
/// Change only the effective user identity while preserving the real identity.
/// Privilege checks and saved-ID rules are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses seteuid(2) on Unix and token adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_euid(
    context: &RuntimeCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = (context, userid);
    not_supported("destack.process.ids.setEuid")
}

/// Set the effective group identifier.
///
/// Set the process primary group identity.
/// Real and effective group update behavior follows host setgid semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses setgid(2) on Unix and token adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_gid(
    context: &RuntimeCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = (context, groupid);
    not_supported("destack.process.ids.setGid")
}

/// Set real, effective, and saved-set group identifiers together.
///
/// Update all group-id classes in one host operation.
/// Privilege checks and immutable-id restrictions follow host kernel rules.
///
/// # Platform
/// Unix and Windows.
/// Uses setresgid(2) on Unix and token adjustment or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_group_ids(
    context: &RuntimeCallContext,
    ids: ProcessGroupIds,
) -> RuntimeResult<()> {
    let _ = (context, ids);
    not_supported("destack.process.ids.setGroupIds")
}

/// Set supplementary group identifiers.
///
/// Replace the calling process supplementary group list.
/// This operation is typically restricted to privileged processes.
///
/// # Platform
/// Unix and Windows.
/// Uses setgroups(2) on Unix and token group adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_groups(
    context: &RuntimeCallContext,
    groups: NativeSlice<GroupId>,
) -> RuntimeResult<()> {
    let _ = (context, groups);
    not_supported("destack.process.ids.setGroups")
}

/// Set the effective user identifier.
///
/// Set the process user identity.
/// Real and effective identity update behavior follows host setuid semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses setuid(2) on Unix and token adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_uid(
    context: &RuntimeCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = (context, userid);
    not_supported("destack.process.ids.setUid")
}

/// Set real, effective, and saved-set user identifiers together.
///
/// Update all user-id classes in one host operation.
/// Privilege checks and immutable-id restrictions follow host kernel rules.
///
/// # Platform
/// Unix and Windows.
/// Uses setresuid(2) on Unix and token adjustment or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_user_ids(
    context: &RuntimeCallContext,
    ids: ProcessUserIds,
) -> RuntimeResult<()> {
    let _ = (context, ids);
    not_supported("destack.process.ids.setUserIds")
}

/// Return the current user identifier.
///
/// Read the real user identity for the calling process.
/// Identity mapping on Windows is best-effort and may require token translation.
///
/// # Platform
/// Unix and Windows.
/// Uses getuid(2) on Unix and token SID translation on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_uid(
    context: &RuntimeCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = UserId(not_supported("destack.process.uid")?);
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return real, effective, and saved-set user identifiers.
///
/// Read all user-id classes for the calling process in one operation.
/// Saved-id availability follows host kernel semantics and can be unsupported on some targets.
///
/// # Platform
/// Unix and Windows.
/// Uses getresuid(2) on Unix and token mapping or `notSupported` on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_user_ids(
    context: &RuntimeCallContext,
    out: *mut ProcessUserIds,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = not_supported("destack.process.userIds")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return a standardized not-supported error for identity operations.
fn not_supported<T>(operation: &'static str) -> RuntimeResult<T> {
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}
