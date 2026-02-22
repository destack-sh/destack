use std::path::PathBuf;

use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    DirentKind, FileMode, FileOffset, FileSize, PathBytes, PathUtf16, Stat, core as core_fs,
};
use crate::platform::resource::{
    DirectoryHandle, FileHandle, ResourceFinalizer, ResourceId, ResourceKind,
};
use crate::platform::{NativeStringRef, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;

/// Directory payload stored in the resource table.
#[derive(Debug, Clone)]
pub(super) struct DirectoryResource {
    /// Directory path.
    pub(super) path: PathBuf,
    /// Directory file descriptor.
    pub(super) fd: RawFd,
}

/// Finalizer that closes a raw file descriptor.
#[derive(Debug)]
pub(super) struct FdFinalizer {
    /// File descriptor to close.
    pub(super) fd: RawFd,
}

impl ResourceFinalizer for FdFinalizer {
    /// Close the file descriptor when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Resolve a raw byte path into a PathBuf.
pub(super) fn resolve_path_bytes(path: PathBytes, name: &str) -> RuntimeResult<PathBuf> {
    // validate the path bytes
    let bytes = unsafe { path.0.as_slice()? };
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed());
    }

    // build the path buffer
    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

/// Resolve a raw byte path into a CString for libc calls.
pub(super) fn resolve_path_bytes_cstring(path: PathBytes, name: &str) -> RuntimeResult<CString> {
    // validate the path bytes
    let bytes = unsafe { path.0.as_slice()? };

    // build the c string
    CString::new(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed()
    })
}

/// Resolve a UTF-16 path into a CString by decoding to UTF-8.
pub(super) fn resolve_path_utf16_cstring(path: PathUtf16, name: &str) -> RuntimeResult<CString> {
    let utf16_units = unsafe { path.0.as_slice()? };
    let decoded = String::from_utf16(utf16_units).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains invalid utf16",
        ))
        .boxed()
    })?;
    CString::new(decoded).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed()
    })
}

/// Resolve a string argument into a CString.
pub(super) fn resolve_name_cstring(name: NativeStringRef, label: &str) -> RuntimeResult<CString> {
    let name = unsafe { name.as_str()? };
    CString::new(name).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "name contains nul byte",
        ))
        .boxed()
    })
}

/// Resolve a resource entry for a file handle.
pub(super) fn file_descriptor(
    context: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<RawFd> {
    // resolve the file resource
    core_fs::require_resource(context, handle.0, ResourceKind::File, "file", |entry| {
        entry.fd().ok_or_else(|| {
            RuntimeError::from(PlatformError::generic(
                None,
                "file handle missing descriptor",
            ))
            .boxed()
        })
    })
}

/// Resolve a directory resource from a handle.
pub(super) fn directory_resource(
    context: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<DirectoryResource> {
    // resolve the directory resource
    core_fs::require_resource(
        context,
        handle.0,
        ResourceKind::Directory,
        "directory",
        |entry| {
            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<DirectoryResource>())
                .cloned()
                .ok_or_else(|| {
                    RuntimeError::from(PlatformError::generic(
                        None,
                        "directory handle missing payload",
                    ))
                    .boxed()
                })
        },
    )
}

/// Resolve a directory handle to its file descriptor.
pub(super) fn directory_descriptor(
    context: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<RawFd> {
    Ok(directory_resource(context, handle)?.fd)
}

/// Resolve a raw byte path into a CString for libc calls.
pub(super) fn path_bytes_to_cstring(path: PathBytes, name: &str) -> RuntimeResult<CString> {
    resolve_path_bytes_cstring(path, name)
}

/// Convert one host mode value into the ABI mode width.
pub(super) fn file_mode_u32(mode: libc::mode_t) -> u32 {
    #[cfg(target_os = "android")]
    {
        mode
    }

    #[cfg(not(target_os = "android"))]
    {
        mode as u32
    }
}

/// Convert one host link-count value into the ABI width.
pub(super) fn file_nlink_u32(link_count: libc::nlink_t) -> u32 {
    #[cfg(target_os = "android")]
    {
        link_count
    }

    #[cfg(not(target_os = "android"))]
    {
        link_count as u32
    }
}

/// Map a libc dirent type to our dirent kind.
pub(super) fn dirent_kind_from_type(kind: u8) -> DirentKind {
    // map the dirent type
    match kind {
        x if x == libc::DT_REG => DirentKind::File,
        x if x == libc::DT_DIR => DirentKind::Directory,
        x if x == libc::DT_LNK => DirentKind::Symlink,
        x if x == libc::DT_BLK => DirentKind::BlockDevice,
        x if x == libc::DT_CHR => DirentKind::CharDevice,
        x if x == libc::DT_FIFO => DirentKind::Fifo,
        x if x == libc::DT_SOCK => DirentKind::Socket,
        _ => DirentKind::Unknown,
    }
}

/// Convert a file offset into a libc off_t.
pub(super) fn offset_to_off_t(offset: FileOffset) -> RuntimeResult<libc::off_t> {
    // cast to off_t
    Ok(offset.0 as libc::off_t)
}

/// Convert libc stat into the ABI Stat shape.
pub(super) fn stat_from_libc(stat: libc::stat) -> Stat {
    let (atime_ns, mtime_ns, ctime_ns, birthtime_ns) = os::stat_times(stat);
    Stat {
        dev: stat_u64(stat.st_dev),
        ino: stat_u64(stat.st_ino),
        mode: FileMode(file_mode_u32(stat.st_mode)),
        nlink: file_nlink_u32(stat.st_nlink),
        uid: stat.st_uid,
        gid: stat.st_gid,
        rdev: stat_u64(stat.st_rdev),
        size: FileSize(stat_u64(stat.st_size)),
        blksize: stat_u64(stat.st_blksize),
        blocks: stat_u64(stat.st_blocks),
        atime_ns,
        mtime_ns,
        ctime_ns,
        birthtime_ns,
    }
}

/// Convert a statfs numeric field into a u64.
pub(super) fn statfs_u64<T>(value: T) -> u64
where
    T: TryInto<u64>,
{
    value.try_into().unwrap_or(0)
}

/// Convert a stat numeric field into a u64.
fn stat_u64<T>(value: T) -> u64
where
    T: TryInto<u64>,
{
    value.try_into().unwrap_or(0)
}

/// Return nanosecond timestamps for stat fields.
/// Build a nanosecond timestamp from seconds + nanoseconds.
pub(super) fn nanos_from_secs_and_nanos(seconds: u64, nanos: u64) -> u64 {
    seconds.saturating_mul(1_000_000_000).saturating_add(nanos)
}

/// Read a symlink target into a byte buffer.
pub(super) fn readlink_bytes_at(dir_fd: RawFd, path: &CString) -> RuntimeResult<Vec<u8>> {
    let mut buffer = vec![0u8; 256];
    loop {
        let result = unsafe {
            libc::readlinkat(
                dir_fd,
                path.as_ptr(),
                buffer.as_mut_ptr() as *mut libc::c_char,
                buffer.len(),
            )
        };
        if result < 0 {
            return Err(core_platform::io_error("readlinkat", None));
        }
        let result = result as usize;
        if result < buffer.len() {
            buffer.truncate(result);
            return Ok(buffer);
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

/// Map a stat mode to a dirent kind.
pub(super) fn dirent_kind_from_mode(mode: libc::mode_t) -> DirentKind {
    // map the mode to a dirent kind
    match mode & libc::S_IFMT {
        libc::S_IFREG => DirentKind::File,
        libc::S_IFDIR => DirentKind::Directory,
        libc::S_IFLNK => DirentKind::Symlink,
        libc::S_IFBLK => DirentKind::BlockDevice,
        libc::S_IFCHR => DirentKind::CharDevice,
        libc::S_IFIFO => DirentKind::Fifo,
        libc::S_IFSOCK => DirentKind::Socket,
        _ => DirentKind::Unknown,
    }
}

/// Convert a nanosecond timestamp into a timespec.
pub(super) fn timespec_from_nanos(nanos: u64) -> libc::timespec {
    // split nanoseconds into seconds and nanos
    libc::timespec {
        tv_sec: (nanos / 1_000_000_000) as libc::time_t,
        tv_nsec: (nanos % 1_000_000_000) as libc::c_long,
    }
}
