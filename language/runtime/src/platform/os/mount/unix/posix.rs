use std::ffi::{CStr, OsString};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::mount::core::MountEntryOwned;
use crate::platform::{PlatformError, core as core_platform};

/// Build one invariant error for impossible host mount row counts.
fn mount_invariant_error(message: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        message,
    ))
    .boxed()
}

/// Read Apple or BSD mount entries from `getfsstat`.
pub(crate) fn read_mount_entries() -> RuntimeResult<Vec<MountEntryOwned>> {
    let count = unsafe { libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT) };
    if count < 0 {
        return Err(core_platform::io_error("getfsstat", None));
    }

    // size one row buffer from the reported count
    let count = usize::try_from(count)
        .map_err(|_| mount_invariant_error("getfsstat returned one negative entry count"))?;
    let mut rows = vec![unsafe { std::mem::zeroed::<libc::statfs>() }; count];
    let size_bytes = rows
        .len()
        .checked_mul(std::mem::size_of::<libc::statfs>())
        .ok_or_else(|| mount_invariant_error("statfs buffer size exceeded usize"))?;
    let count = unsafe { libc::getfsstat(rows.as_mut_ptr(), size_bytes as _, libc::MNT_NOWAIT) };
    if count < 0 {
        return Err(core_platform::io_error("getfsstat", None));
    }

    // trim to the actual row count
    let count = usize::try_from(count)
        .map_err(|_| mount_invariant_error("getfsstat returned one negative entry count"))?;
    rows.truncate(count);

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let source = string_from_array(&row.f_mntfromname, "source")?;
        let target = path_from_array(&row.f_mntonname)?;
        let file_system = string_from_array(&row.f_fstypename, "file system")?;

        entries.push(MountEntryOwned {
            source: Some(source),
            target,
            file_system: Some(file_system),
            host_flags: Some(row.f_flags as u64),
        });
    }

    Ok(entries)
}

/// Decode one fixed-size host string field as UTF-8 text.
fn string_from_array(buffer: &[libc::c_char], field: &str) -> RuntimeResult<String> {
    let value = unsafe { CStr::from_ptr(buffer.as_ptr()) };
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

/// Decode one fixed-size host path field as a native path.
fn path_from_array(buffer: &[libc::c_char]) -> RuntimeResult<PathBuf> {
    let value = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    let bytes = value.to_bytes().to_vec();

    Ok(PathBuf::from(OsString::from_vec(bytes)))
}
