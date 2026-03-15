use std::ffi::{CStr, CString, OsString};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::mount::core::MountEntryOwned;
use crate::platform::{PlatformError, core as core_platform};

/// Path for the active process mount table snapshot.
const PROC_MOUNTS_PATH: &str = "/proc/self/mounts";

/// Read Linux or Android mount entries from procfs.
pub(crate) fn read_mount_entries() -> RuntimeResult<Vec<MountEntryOwned>> {
    let mounts_path = CString::new(PROC_MOUNTS_PATH).expect("proc mounts path should be valid");
    let mode = CString::new("r").expect("mntent mode should be valid");
    let file = unsafe { libc::setmntent(mounts_path.as_ptr(), mode.as_ptr()) };
    if file.is_null() {
        return Err(core_platform::io_error("setmntent", None));
    }

    let mut entries = Vec::new();

    loop {
        core_platform::set_errno(0);
        let entry = unsafe { libc::getmntent(file) };
        if entry.is_null() {
            let errno = core_platform::get_errno();
            unsafe {
                libc::endmntent(file);
            }

            if errno != 0 {
                return Err(core_platform::io_error_with_errno("getmntent", errno, None));
            }

            break;
        }

        let entry = unsafe { &*entry };
        let source = string_from_c_path(entry.mnt_fsname, "source")?;
        let target = path_from_c_path(entry.mnt_dir)?;
        let file_system = string_from_c_path(entry.mnt_type, "file system")?;
        let flags = parse_mnt_options(entry.mnt_opts);

        entries.push(MountEntryOwned {
            source: Some(source),
            target,
            file_system: Some(file_system),
            host_flags: Some(flags),
        });
    }

    Ok(entries)
}

/// Decode one host C string field as UTF-8 text.
fn string_from_c_path(pointer: *const libc::c_char, field: &str) -> RuntimeResult<String> {
    let value = unsafe { CStr::from_ptr(pointer) };
    let value = value.to_str().map_err(|_| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some("destack.os.mount.list".to_string()),
            None,
            format!("{field} contained invalid utf8"),
        ))
        .boxed()
    })?;

    Ok(value.to_string())
}

/// Decode one host C path field as a native path.
fn path_from_c_path(pointer: *const libc::c_char) -> RuntimeResult<PathBuf> {
    let value = unsafe { CStr::from_ptr(pointer) };
    let bytes = value.to_bytes().to_vec();

    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

/// Decode one Linux mount option string into a host flag bitmask.
fn parse_mnt_options(pointer: *const libc::c_char) -> u64 {
    let value = unsafe { CStr::from_ptr(pointer) };
    let value = value.to_string_lossy();
    let mut flags = 0u64;

    for option in value.split(',') {
        match option {
            "ro" => flags |= libc::MS_RDONLY as u64,
            "nosuid" => flags |= libc::MS_NOSUID as u64,
            "nodev" => flags |= libc::MS_NODEV as u64,
            "noexec" => flags |= libc::MS_NOEXEC as u64,
            "sync" => flags |= libc::MS_SYNCHRONOUS as u64,
            "dirsync" => flags |= libc::MS_DIRSYNC as u64,
            "mand" => flags |= libc::MS_MANDLOCK as u64,
            "noatime" => flags |= libc::MS_NOATIME as u64,
            "nodiratime" => flags |= libc::MS_NODIRATIME as u64,
            "relatime" => flags |= libc::MS_RELATIME as u64,
            "strictatime" => flags |= libc::MS_STRICTATIME as u64,
            "lazytime" => flags |= libc::MS_LAZYTIME as u64,
            "bind" => flags |= libc::MS_BIND as u64,
            "rbind" => flags |= libc::MS_BIND as u64 | libc::MS_REC as u64,
            "remount" => flags |= libc::MS_REMOUNT as u64,
            _ => {}
        }
    }

    flags
}
