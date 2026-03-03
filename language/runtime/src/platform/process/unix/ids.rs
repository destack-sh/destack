#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{NativeArray, PlatformError};
use crate::runtime::{NativeSlice, NativeStringRef, NativeStringSlice};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions, ProcessStdio,
    ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
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
    _context: &BindingCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = GroupId(unsafe { libc::getegid() as u32 });
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
    _context: &BindingCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = UserId(unsafe { libc::geteuid() as u32 });
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
    _context: &BindingCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = GroupId(unsafe { libc::getgid() as u32 });
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
    _context: &BindingCallContext,
    out: *mut ProcessGroupIds,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    let value = {
        let mut real: libc::gid_t = 0;
        let mut effective: libc::gid_t = 0;
        let mut saved: libc::gid_t = 0;
        let result = unsafe { libc::getresgid(&mut real, &mut effective, &mut saved) };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read group ids: {error}"
            )))
            .boxed());
        }

        ProcessGroupIds {
            real: GroupId(real as u32),
            effective: GroupId(effective as u32),
            saved: GroupId(saved as u32),
        }
    };
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let value = {
        let real = unsafe { libc::getgid() as u32 };
        let effective = unsafe { libc::getegid() as u32 };

        ProcessGroupIds {
            real: GroupId(real),
            effective: GroupId(effective),
            saved: GroupId(effective),
        }
    };
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
    context: &BindingCallContext,
    out: *mut NativeSlice<GroupId>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
    if count < 0 {
        let error = std::io::Error::last_os_error();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read supplementary groups: {error}"
        )))
        .boxed());
    }

    let mut groups = vec![0 as libc::gid_t; count as usize];
    let result = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
    if result < 0 {
        let error = std::io::Error::last_os_error();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read supplementary groups: {error}"
        )))
        .boxed());
    }

    let groups = groups.into_iter().map(GroupId).collect::<Vec<_>>();

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
    _context: &BindingCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = ProcessId(unsafe { libc::getpid() as u32 });
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
    _context: &BindingCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = ProcessId(unsafe { libc::getppid() as u32 });
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
    _context: &BindingCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let result = unsafe { libc::setegid(groupid.0 as libc::gid_t) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "setegid",
            format!("failed to set effective group id to {}", groupid.0),
        ));
    }

    Ok(())
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
    _context: &BindingCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    let result = unsafe { libc::seteuid(userid.0 as libc::uid_t) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "seteuid",
            format!("failed to set effective user id to {}", userid.0),
        ));
    }

    Ok(())
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
    _context: &BindingCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let result = unsafe { libc::setgid(groupid.0 as libc::gid_t) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "setgid",
            format!("failed to set group id to {}", groupid.0),
        ));
    }

    Ok(())
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
    _context: &BindingCallContext,
    ids: ProcessGroupIds,
) -> RuntimeResult<()> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let result = unsafe {
            libc::setresgid(
                ids.real.0 as libc::gid_t,
                ids.effective.0 as libc::gid_t,
                ids.saved.0 as libc::gid_t,
            )
        };
        if result != 0 {
            return Err(core_process::process_last_error(
                "setresgid",
                format!("failed to set group ids to {ids:?}"),
            ));
        }

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if ids.saved != ids.effective {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "ids",
                "saved group id must match effective group id on this platform",
            ))
            .boxed());
        }

        let result =
            unsafe { libc::setregid(ids.real.0 as libc::gid_t, ids.effective.0 as libc::gid_t) };
        if result != 0 {
            return Err(core_process::process_last_error(
                "setregid",
                format!("failed to set group ids to {ids:?}"),
            ));
        }

        Ok(())
    }
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
    _context: &BindingCallContext,
    groups: NativeSlice<GroupId>,
) -> RuntimeResult<()> {
    let groups = unsafe { groups.as_slice()? };
    let mut raw_groups = Vec::with_capacity(groups.len());
    for group in groups {
        raw_groups.push(group.0 as libc::gid_t);
    }

    let count = i32::try_from(raw_groups.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "groups",
            "too many supplementary groups",
        ))
        .boxed()
    })?;
    let result = unsafe { libc::setgroups(count as _, raw_groups.as_ptr()) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "setgroups",
            "failed to set supplementary groups",
        ));
    }

    Ok(())
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
    _context: &BindingCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    let result = unsafe { libc::setuid(userid.0 as libc::uid_t) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "setuid",
            format!("failed to set user id to {}", userid.0),
        ));
    }

    Ok(())
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
    _context: &BindingCallContext,
    ids: ProcessUserIds,
) -> RuntimeResult<()> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let result = unsafe {
            libc::setresuid(
                ids.real.0 as libc::uid_t,
                ids.effective.0 as libc::uid_t,
                ids.saved.0 as libc::uid_t,
            )
        };
        if result != 0 {
            return Err(core_process::process_last_error(
                "setresuid",
                format!("failed to set user ids to {ids:?}"),
            ));
        }

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if ids.saved != ids.effective {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "ids",
                "saved user id must match effective user id on this platform",
            ))
            .boxed());
        }

        let result =
            unsafe { libc::setreuid(ids.real.0 as libc::uid_t, ids.effective.0 as libc::uid_t) };
        if result != 0 {
            return Err(core_process::process_last_error(
                "setreuid",
                format!("failed to set user ids to {ids:?}"),
            ));
        }

        Ok(())
    }
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
    _context: &BindingCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = UserId(unsafe { libc::getuid() as u32 });
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
    _context: &BindingCallContext,
    out: *mut ProcessUserIds,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    let value = {
        let mut real: libc::uid_t = 0;
        let mut effective: libc::uid_t = 0;
        let mut saved: libc::uid_t = 0;
        let result = unsafe { libc::getresuid(&mut real, &mut effective, &mut saved) };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read user ids: {error}"
            )))
            .boxed());
        }

        ProcessUserIds {
            real: UserId(real as u32),
            effective: UserId(effective as u32),
            saved: UserId(saved as u32),
        }
    };

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let value = {
        let real = unsafe { libc::getuid() as u32 };
        let effective = unsafe { libc::geteuid() as u32 };

        ProcessUserIds {
            real: UserId(real),
            effective: UserId(effective),
            saved: UserId(effective),
        }
    };

    unsafe {
        *out = value;
    }

    Ok(())
}
