use std::path::PathBuf;

use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetVolumeInformationW,
    GetVolumePathNamesForVolumeNameW,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::mount::core::MountEntryOwned;

/// Initial wide buffer capacity used for Windows volume enumeration.
const INITIAL_WIDE_BUFFER_UNITS: usize = 260;

/// Read the current host mount table on Windows.
pub(crate) fn read_mount_entries() -> RuntimeResult<Vec<MountEntryOwned>> {
    let mut volume_buffer = vec![0u16; INITIAL_WIDE_BUFFER_UNITS];
    let handle =
        unsafe { FindFirstVolumeW(volume_buffer.as_mut_ptr(), volume_buffer.len() as u32) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(core_platform::io_error("FindFirstVolumeW"));
    }

    let mut entries = Vec::new();

    loop {
        let volume_name = trim_trailing_nul(&volume_buffer);
        let source = core_platform::string_from_wide("source", volume_name)?;
        let (file_system, flags) = volume_information(volume_name)?;
        let mount_paths = volume_mount_paths(volume_name)?;

        for target in mount_paths {
            entries.push(MountEntryOwned {
                source: Some(source.clone()),
                target,
                file_system: if file_system.is_empty() {
                    None
                } else {
                    Some(file_system.clone())
                },
                host_flags: Some(flags),
            });
        }

        volume_buffer.fill(0);
        let status = unsafe {
            FindNextVolumeW(
                handle,
                volume_buffer.as_mut_ptr(),
                volume_buffer.len() as u32,
            )
        };
        if status != 0 {
            continue;
        }

        let error = core_platform::last_error_code();
        if error == ERROR_NO_MORE_FILES as i32 {
            break;
        }

        unsafe { FindVolumeClose(handle) };
        return Err(core_platform::io_error_with_code("FindNextVolumeW", error));
    }

    unsafe { FindVolumeClose(handle) };

    Ok(entries)
}

/// Read one file system name and host flag set for a volume.
fn volume_information(volume_name: &[u16]) -> RuntimeResult<(String, u64)> {
    let mut file_system_name = vec![0u16; INITIAL_WIDE_BUFFER_UNITS];
    let mut flags = 0u32;
    let status = unsafe {
        GetVolumeInformationW(
            with_trailing_nul(volume_name).as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut flags,
            file_system_name.as_mut_ptr(),
            file_system_name.len() as u32,
        )
    };

    if status != 0 {
        let file_system_name = trim_trailing_nul(&file_system_name);
        let file_system_name = core_platform::string_from_wide("fileSystem", file_system_name)?;

        return Ok((file_system_name, flags as u64));
    }

    let error = core_platform::last_error_code();

    // field availability
    if error == windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED as i32
        || error == windows_sys::Win32::Foundation::ERROR_NOT_READY as i32
    {
        return Ok((String::new(), 0));
    }

    Err(core_platform::io_error_with_code(
        "GetVolumeInformationW",
        error,
    ))
}

/// Read all mount paths that currently reference one volume.
fn volume_mount_paths(volume_name: &[u16]) -> RuntimeResult<Vec<PathBuf>> {
    let mut capacity = INITIAL_WIDE_BUFFER_UNITS;

    loop {
        let mut buffer = vec![0u16; capacity];
        let mut required = 0u32;
        let status = unsafe {
            GetVolumePathNamesForVolumeNameW(
                with_trailing_nul(volume_name).as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                &mut required,
            )
        };

        if status != 0 {
            let mut paths = Vec::new();
            for path in split_wide_multi_string(&buffer) {
                let path = core_platform::pathbuf_from_utf16("target", path)?;
                paths.push(path);
            }

            return Ok(paths);
        }

        let error = core_platform::last_error_code();
        if error == ERROR_MORE_DATA as i32 && (required as usize) > capacity {
            capacity = required as usize;
            continue;
        }

        return Err(core_platform::io_error_with_code(
            "GetVolumePathNamesForVolumeNameW",
            error,
        ));
    }
}

/// Return one slice without its trailing nul terminator.
fn trim_trailing_nul(buffer: &[u16]) -> &[u16] {
    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());

    &buffer[..end]
}

/// Return one temporary nul-terminated copy of a wide string.
fn with_trailing_nul(buffer: &[u16]) -> Vec<u16> {
    let mut value = buffer.to_vec();
    value.push(0);

    value
}

/// Split one Windows multi-string buffer into individual UTF-16 strings.
fn split_wide_multi_string(buffer: &[u16]) -> Vec<&[u16]> {
    let mut values = Vec::new();
    let mut start = 0usize;

    while start < buffer.len() {
        let Some(relative_end) = buffer[start..].iter().position(|unit| *unit == 0) else {
            break;
        };
        let end = start + relative_end;
        if end == start {
            break;
        }

        values.push(&buffer[start..end]);
        start = end + 1;
    }

    values
}
