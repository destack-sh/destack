#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeSlice;
use crate::platform::process::core as core_process;

use crate::runtime::BindingCallContext;

use crate::platform::process::{GroupId, ProcessGroupIds, ProcessId, ProcessUserIds, UserId};

/// Return the effective group identifier.
pub(crate) unsafe fn destack_process_egid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_euid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_gid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_group_ids(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_groups(
    binding: &BindingCallContext,
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
        *out = binding.store_slice(groups);
    }

    Ok(())
}

/// Return the current process identifier.
pub(crate) unsafe fn destack_process_pid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_ppid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_set_egid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_set_euid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_set_gid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_set_group_ids(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_set_groups(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_set_uid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_set_user_ids(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_uid(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_user_ids(
    _binding: &BindingCallContext,
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
