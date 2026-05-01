use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

/// Resolve one mount point into one volume name path (for example, `\\?\Volume{GUID}\`).
pub(super) fn volume_name_from_mount_point<S: AsRef<OsStr>>(
    mount_point: S,
) -> io::Result<OsString> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Storage::FileSystem::GetVolumeNameForVolumeMountPointW;

    const BUFFER_SIZE: u32 = 64;
    let mount_point: Vec<u16> = mount_point.as_ref().encode_wide().chain(Some(0)).collect();
    let mut buffer = vec![0; BUFFER_SIZE as usize];

    // win32 search to read volume name for one mount point
    let success = unsafe {
        GetVolumeNameForVolumeMountPointW(mount_point.as_ptr(), buffer.as_mut_ptr(), BUFFER_SIZE)
    };
    if success == 0 {
        return Err(io::Error::from_raw_os_error(
            unsafe { GetLastError() } as i32
        ));
    }

    let length = buffer.iter().position(|&value| value == 0).unwrap_or(0);
    Ok(OsString::from_wide(&buffer[..length]))
}

/// Convert one rooted path into its DOS device path variant.
pub(super) fn get_dos_device_path<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let path = path.as_ref();
    assert!(path.has_root(), "expected path with root");

    // volume mount APIs require one trailing backslash
    let mut root = OsString::from(path.components().next().unwrap().as_os_str());
    root.push(r"\");

    // map mount root to its volume name
    let mut volume_name_root: Vec<u16> =
        volume_name_from_mount_point(root)?.encode_wide().collect();
    if volume_name_root.starts_with(&[92, 92, 63, 92]) {
        // replace \\?\ with \\.\ for better compatibility across io operations
        volume_name_root[2] = u16::from(b'.');
    }

    // append original trailing components to the resolved volume root
    let mut dos_device_path = PathBuf::from(OsString::from_wide(&volume_name_root));
    dos_device_path.extend(path.components().skip(1));

    Ok(dos_device_path)
}

/// Ensure DOS device conversion roundtrips through canonicalize consistently.
#[test]
fn test_get_dos_device_path() {
    let root = super::fixture_root();
    let dos_device_path = get_dos_device_path(&root).unwrap();

    let canonical_dos_device_path = std::fs::canonicalize(&dos_device_path).unwrap();
    let canonical_root = std::fs::canonicalize(&root).unwrap();

    assert_eq!(canonical_dos_device_path, canonical_root);
}
