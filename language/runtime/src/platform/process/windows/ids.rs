#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::{PlatformError, PlatformErrorCode};

use crate::runtime::BindingCallContext;

use crate::platform::process::{GroupId, ProcessGroupIds, ProcessId, ProcessUserIds, UserId};
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

/// Read the parent process identifier for the current process.
fn current_parent_pid() -> RuntimeResult<ProcessId> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        let error = std::io::Error::last_os_error();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to create process snapshot: {error}",
        )))
        .boxed());
    }

    let current_pid = unsafe { GetCurrentProcessId() };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        cntUsage: 0,
        th32ProcessID: 0,
        th32DefaultHeapID: 0,
        th32ModuleID: 0,
        cntThreads: 0,
        th32ParentProcessID: 0,
        pcPriClassBase: 0,
        dwFlags: 0,
        szExeFile: [0; 260],
    };

    let mut result = Err(RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some("CreateToolhelp32Snapshot".to_string()),
        None,
        format!("failed to locate current process entry for pid {current_pid}"),
    ))
    .boxed());
    let mut process_entry = unsafe { Process32FirstW(snapshot, &mut entry) };
    while process_entry != 0 {
        if entry.th32ProcessID == current_pid {
            result = Ok(ProcessId(entry.th32ParentProcessID));
            break;
        }

        process_entry = unsafe { Process32NextW(snapshot, &mut entry) };
    }

    unsafe {
        CloseHandle(snapshot);
    }

    result
}

/// Return the effective group identifier.
pub(crate) unsafe fn destack_process_egid(
    _binding: &BindingCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = GroupId(not_supported("destack.process.ids.egid")?);
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
    let value = UserId(not_supported("destack.process.ids.euid")?);
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
    let value = GroupId(not_supported("destack.process.ids.gid")?);
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
    let value = not_supported("destack.process.ids.groupIds")?;
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
    let groups = not_supported("destack.process.ids.groups")?;
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
    let value = ProcessId(unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() });
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
    let value = current_parent_pid()?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Set the effective group identifier only.
pub(crate) unsafe fn destack_process_set_egid(
    binding: &BindingCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = (binding, groupid);
    not_supported("destack.process.ids.setEgid")
}

/// Set the effective user identifier only.
pub(crate) unsafe fn destack_process_set_euid(
    binding: &BindingCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = (binding, userid);
    not_supported("destack.process.ids.setEuid")
}

/// Set the effective group identifier.
pub(crate) unsafe fn destack_process_set_gid(
    binding: &BindingCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = (binding, groupid);
    not_supported("destack.process.ids.setGid")
}

/// Set real, effective, and saved-set group identifiers together.
pub(crate) unsafe fn destack_process_set_group_ids(
    binding: &BindingCallContext,
    ids: ProcessGroupIds,
) -> RuntimeResult<()> {
    let _ = (binding, ids);
    not_supported("destack.process.ids.setGroupIds")
}

/// Set supplementary group identifiers.
pub(crate) unsafe fn destack_process_set_groups(
    binding: &BindingCallContext,
    groups: NativeSlice<GroupId>,
) -> RuntimeResult<()> {
    let _ = (binding, groups);
    not_supported("destack.process.ids.setGroups")
}

/// Set the effective user identifier.
pub(crate) unsafe fn destack_process_set_uid(
    binding: &BindingCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = (binding, userid);
    not_supported("destack.process.ids.setUid")
}

/// Set real, effective, and saved-set user identifiers together.
pub(crate) unsafe fn destack_process_set_user_ids(
    binding: &BindingCallContext,
    ids: ProcessUserIds,
) -> RuntimeResult<()> {
    let _ = (binding, ids);
    not_supported("destack.process.ids.setUserIds")
}

/// Return the current user identifier.
pub(crate) unsafe fn destack_process_uid(
    _binding: &BindingCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = UserId(not_supported("destack.process.ids.uid")?);
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
    let value = not_supported("destack.process.ids.userIds")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return a standardized not-supported error for identity operations.
fn not_supported<T>(operation: &'static str) -> RuntimeResult<T> {
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}
