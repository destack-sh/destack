#[cfg(not(target_os = "android"))]
use std::ffi::CStr;
#[cfg(not(target_os = "android"))]
use std::ffi::CString;
use std::ffi::OsString;
#[cfg(target_os = "android")]
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[cfg(not(target_os = "android"))]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(not(target_os = "android"))]
use crate::platform::PlatformError;
use crate::platform::core as core_platform;
#[cfg(not(target_os = "android"))]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::mount::core::MountEntryOwned;

/// Path for the active process mount table snapshot.
const PROC_MOUNTS_PATH: &str = "/proc/self/mounts";

/// Read Linux or Android mount entries from procfs.
#[cfg(not(target_os = "android"))]
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

/// Read Linux or Android mount entries from procfs.
#[cfg(target_os = "android")]
pub(crate) fn read_mount_entries() -> RuntimeResult<Vec<MountEntryOwned>> {
    read_android_mount_entries()
}

/// Read Android mount entries by parsing `/proc/self/mounts` directly.
#[cfg(target_os = "android")]
fn read_android_mount_entries() -> RuntimeResult<Vec<MountEntryOwned>> {
    // read one procfs mount table snapshot
    let mounts = fs::read_to_string(PROC_MOUNTS_PATH)
        .map_err(|error| core_platform::io_operation_error("read", None, error.to_string()))?;
    let mut entries = Vec::new();

    // parse one procfs line into one owned mount entry
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let Some(source) = fields.next() else {
            continue;
        };
        let Some(target) = fields.next() else {
            continue;
        };
        let Some(file_system) = fields.next() else {
            continue;
        };
        let Some(options) = fields.next() else {
            continue;
        };

        let source = decode_proc_mount_text(source);
        let target = decode_proc_mount_path(target);
        let file_system = decode_proc_mount_text(file_system);
        let flags = parse_proc_mount_options(options);

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
#[cfg(not(target_os = "android"))]
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
#[cfg(not(target_os = "android"))]
fn path_from_c_path(pointer: *const libc::c_char) -> RuntimeResult<PathBuf> {
    let value = unsafe { CStr::from_ptr(pointer) };
    let bytes = value.to_bytes().to_vec();

    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

/// Decode one escaped procfs text field into UTF-8 text.
#[cfg(target_os = "android")]
fn decode_proc_mount_text(value: &str) -> String {
    String::from_utf8_lossy(&decode_proc_mount_bytes(value)).into_owned()
}

/// Decode one escaped procfs path field into one native path.
#[cfg(target_os = "android")]
fn decode_proc_mount_path(value: &str) -> PathBuf {
    let bytes = decode_proc_mount_bytes(value);

    PathBuf::from(OsString::from_vec(bytes))
}

/// Decode one escaped procfs field into raw bytes.
#[cfg(target_os = "android")]
fn decode_proc_mount_bytes(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    // decode one `\ooo` escape when present
    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 3 < bytes.len() {
            let first = bytes[index + 1];
            let second = bytes[index + 2];
            let third = bytes[index + 3];
            if first.is_ascii_digit() && second.is_ascii_digit() && third.is_ascii_digit() {
                let first_digit = first - b'0';
                let second_digit = second - b'0';
                let third_digit = third - b'0';
                let decoded_byte = (first_digit << 6) | (second_digit << 3) | third_digit;

                decoded.push(decoded_byte);
                index += 4;
                continue;
            }
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    decoded
}

/// Decode one Linux mount option string into a host flag bitmask.
#[cfg(not(target_os = "android"))]
fn parse_mnt_options(pointer: *const libc::c_char) -> u64 {
    let value = unsafe { CStr::from_ptr(pointer) };
    let value = value.to_string_lossy();
    let mut flags = 0u64;

    for option in value.split(',') {
        match option {
            "ro" => flags |= libc::MS_RDONLY,
            "nosuid" => flags |= libc::MS_NOSUID,
            "nodev" => flags |= libc::MS_NODEV,
            "noexec" => flags |= libc::MS_NOEXEC,
            "sync" => flags |= libc::MS_SYNCHRONOUS,
            "dirsync" => flags |= libc::MS_DIRSYNC,
            "mand" => flags |= libc::MS_MANDLOCK,
            "noatime" => flags |= libc::MS_NOATIME,
            "nodiratime" => flags |= libc::MS_NODIRATIME,
            "relatime" => flags |= libc::MS_RELATIME,
            "strictatime" => flags |= libc::MS_STRICTATIME,
            "lazytime" => flags |= libc::MS_LAZYTIME,
            "bind" => flags |= libc::MS_BIND,
            "rbind" => flags |= libc::MS_BIND | libc::MS_REC,
            "remount" => flags |= libc::MS_REMOUNT,
            _ => {}
        }
    }

    flags
}

/// Decode one Android procfs mount option string into a host flag bitmask.
#[cfg(target_os = "android")]
fn parse_proc_mount_options(value: &str) -> u64 {
    let mut flags = 0u64;

    for option in value.split(',') {
        match option {
            "ro" => flags |= libc::MS_RDONLY,
            "nosuid" => flags |= libc::MS_NOSUID,
            "nodev" => flags |= libc::MS_NODEV,
            "noexec" => flags |= libc::MS_NOEXEC,
            "sync" => flags |= libc::MS_SYNCHRONOUS,
            "dirsync" => flags |= libc::MS_DIRSYNC,
            "mand" => flags |= libc::MS_MANDLOCK,
            "noatime" => flags |= libc::MS_NOATIME,
            "nodiratime" => flags |= libc::MS_NODIRATIME,
            "relatime" => flags |= libc::MS_RELATIME,
            "strictatime" => flags |= libc::MS_STRICTATIME,
            "lazytime" => flags |= libc::MS_LAZYTIME,
            "bind" => flags |= libc::MS_BIND,
            "rbind" => flags |= libc::MS_BIND | libc::MS_REC,
            "remount" => flags |= libc::MS_REMOUNT,
            _ => {}
        }
    }

    flags
}
