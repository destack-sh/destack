use std::path::PathBuf;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    AccessMode, AllocFlags, AtFlags, CopyFlags, Dirent, DirentKind, FileAdvice, FileLockFlags,
    FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags, OpenFlags,
    OpenOptions, PathBytes, PathBytesAbi, PathUtf16, RenameFlags, SeekWhence, Stat, StatFs,
    SymlinkType, SyncFlags, XattrFlags, core as core_fs,
};
use crate::platform::resource::{
    DirectoryHandle, FileHandle, ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind,
    SocketHandle,
};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, PlatformError, core as core_platform,
    net as platform_net,
};
use crate::runtime::RuntimeCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "linux.rs"]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
use linux as os;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;

#[cfg(target_os = "ios")]
#[path = "ios.rs"]
mod ios;
#[cfg(target_os = "ios")]
use ios as os;

#[cfg(target_os = "freebsd")]
#[path = "freebsd.rs"]
mod freebsd;
#[cfg(target_os = "freebsd")]
use freebsd as os;

#[cfg(target_os = "openbsd")]
#[path = "openbsd.rs"]
mod openbsd;
#[cfg(target_os = "openbsd")]
use openbsd as os;

#[cfg(target_os = "netbsd")]
#[path = "netbsd.rs"]
mod netbsd;
#[cfg(target_os = "netbsd")]
use netbsd as os;

#[cfg(target_os = "dragonfly")]
#[path = "dragonfly.rs"]
mod dragonfly;
#[cfg(target_os = "dragonfly")]
use dragonfly as os;

#[path = "statfs.rs"]
mod statfs;

/// Directory payload stored in the resource table.
#[derive(Debug, Clone)]
struct DirectoryResource {
    /// Directory path.
    path: PathBuf,
    /// Directory file descriptor.
    fd: RawFd,
}

/// Finalizer that closes a raw file descriptor.
#[derive(Debug)]
struct FdFinalizer {
    /// File descriptor to close.
    fd: RawFd,
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
fn resolve_path_bytes(path: PathBytes, name: &str) -> RuntimeResult<PathBuf> {
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
fn resolve_path_bytes_cstring(path: PathBytes, name: &str) -> RuntimeResult<CString> {
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
fn resolve_path_utf16_cstring(path: PathUtf16, name: &str) -> RuntimeResult<CString> {
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
fn resolve_name_cstring(name: NativeStringRef, label: &str) -> RuntimeResult<CString> {
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
fn file_descriptor(context: &RuntimeCallContext, handle: FileHandle) -> RuntimeResult<RawFd> {
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
fn directory_resource(
    context: &RuntimeCallContext,
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

/// Map a libc dirent type to our dirent kind.
fn dirent_kind_from_type(kind: u8) -> DirentKind {
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
fn offset_to_off_t(offset: FileOffset) -> RuntimeResult<libc::off_t> {
    // cast to off_t
    Ok(offset.0 as libc::off_t)
}

/// Convert libc stat into the ABI Stat shape.
fn stat_from_libc(stat: libc::stat) -> Stat {
    let (atime_ns, mtime_ns, ctime_ns, birthtime_ns) = os::stat_times(stat);
    Stat {
        dev: stat_u64(stat.st_dev),
        ino: stat_u64(stat.st_ino),
        mode: FileMode(stat.st_mode as u32),
        nlink: stat.st_nlink as u32,
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
fn readlink_bytes_at(dir_fd: RawFd, path: &CString) -> RuntimeResult<Vec<u8>> {
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
fn dirent_kind_from_mode(mode: libc::mode_t) -> DirentKind {
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
fn timespec_from_nanos(nanos: u64) -> libc::timespec {
    // split nanoseconds into seconds and nanos
    libc::timespec {
        tv_sec: (nanos / 1_000_000_000) as libc::time_t,
        tv_nsec: (nanos % 1_000_000_000) as libc::c_long,
    }
}

/// Binding implementation for `destack.fs.access`.
/// Binding implementation for `destack.fs.accessBytes`.
pub(crate) unsafe fn destack_fs_access_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // run the access check on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::access(c_path.as_ptr(), mode.0 as libc::c_int) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("access", None))
    }

    // report unsupported access checks on non-unix platforms
}

/// Binding implementation for `destack.fs.accessUtf16`.
pub(crate) unsafe fn destack_fs_access_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: AccessMode,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported access checks on non-windows platforms
    {
        let _ = (path, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessUtf16")).boxed())
    }

    // run the access check on windows platforms
}

/// Binding implementation for `destack.fs.chmod`.
/// Binding implementation for `destack.fs.chmodBytes`.
pub(crate) unsafe fn destack_fs_chmod_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // apply permissions on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::chmod(c_path.as_ptr(), mode.0 as libc::mode_t) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("chmod", None))
    }

    // report unsupported chmod calls on non-unix platforms
}

/// Binding implementation for `destack.fs.chmodUtf16`.
pub(crate) unsafe fn destack_fs_chmod_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported chmod calls on non-windows platforms
    {
        let _ = (path, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmodUtf16")).boxed())
    }

    // apply permissions on windows platforms
}

/// Binding implementation for `destack.fs.fchmodat`.
/// Binding implementation for `destack.fs.fchmodatBytes`.
pub(crate) unsafe fn destack_fs_fchmodat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // apply permissions relative to the directory on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe {
            libc::fchmodat(
                resource.fd,
                c_path.as_ptr(),
                mode.0 as libc::mode_t,
                flags.0 as libc::c_int,
            )
        };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("fchmodat", None))
    }

    // report unsupported fchmodat calls on non-unix platforms
}

/// Binding implementation for `destack.fs.fchmodatUtf16`.
pub(crate) unsafe fn destack_fs_fchmodat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported fchmodat calls on non-windows platforms
    {
        let _ = (dir, path, mode, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmodatUtf16")).boxed())
    }

    // apply permissions on windows platforms
}

/// Binding implementation for `destack.fs.chown`.
/// Binding implementation for `destack.fs.chownBytes`.
pub(crate) unsafe fn destack_fs_chown_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // apply ownership on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::chown(c_path.as_ptr(), uid, gid) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("chown", None))
    }

    // report unsupported chown calls on non-unix platforms
}

/// Binding implementation for `destack.fs.chownUtf16`.
pub(crate) unsafe fn destack_fs_chown_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported chown calls on non-windows platforms
    {
        let _ = (path, uid, gid);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownUtf16")).boxed())
    }

    // apply ownership on windows platforms
}

/// Binding implementation for `destack.fs.fchownat`.
/// Binding implementation for `destack.fs.fchownatBytes`.
pub(crate) unsafe fn destack_fs_fchownat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // apply ownership relative to the directory on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::fchownat(resource.fd, c_path.as_ptr(), uid, gid, flags.0 as i32) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("fchownat", None))
    }

    // report unsupported fchownat calls on non-unix platforms
}

/// Binding implementation for `destack.fs.fchownatUtf16`.
pub(crate) unsafe fn destack_fs_fchownat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported fchownat calls on non-windows platforms
    {
        let _ = (dir, path, uid, gid, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchownatUtf16")).boxed())
    }

    // apply ownership on windows platforms
}

/// Binding implementation for `destack.fs.close`.
pub(crate) unsafe fn destack_fs_close(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // validate the handle kind
    let is_file = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| entry.kind == ResourceKind::File)
        .unwrap_or(false);
    if !is_file {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown file handle",
        ))
        .boxed());
    }

    // remove the resource and close the descriptor
    if !context.runtime().resources.remove_and_finalize(handle.0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown file handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Binding implementation for `destack.fs.closedir`.
pub(crate) unsafe fn destack_fs_closedir(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // validate the handle kind
    let is_directory = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| entry.kind == ResourceKind::Directory)
        .unwrap_or(false);
    if !is_directory {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown directory handle",
        ))
        .boxed());
    }

    // remove the resource entry
    if !context.runtime().resources.remove_and_finalize(handle.0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown directory handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Binding implementation for `destack.fs.copyfile`.
/// Binding implementation for `destack.fs.copyfileBytes`.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // resolve the source and destination paths
    let from_path = resolve_path_bytes(from, "from")?;
    let to_path = resolve_path_bytes(to, "to")?;

    // reject unsupported flags
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported copyfile flags",
        ))
        .boxed());
    }

    // open the source file
    let from_c = CString::new(from_path.as_os_str().as_bytes()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "from",
            "path contains nul byte",
        ))
        .boxed()
    })?;
    let src_fd = unsafe { libc::open(from_c.as_ptr(), libc::O_RDONLY) };
    if src_fd < 0 {
        return Err(core_platform::io_error(
            "open",
            Some(from_path.to_string_lossy().as_ref()),
        ));
    }

    // ensure the source descriptor is closed
    struct FdGuard(RawFd);
    impl Drop for FdGuard {
        fn drop(&mut self) {
            unsafe {
                libc::close(self.0);
            }
        }
    }
    let _src_guard = FdGuard(src_fd);

    // capture the source mode for the destination
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    let rc = unsafe { libc::fstat(src_fd, &mut stat) };
    if rc != 0 {
        return Err(core_platform::io_error(
            "fstat",
            Some(from_path.to_string_lossy().as_ref()),
        ));
    }
    let mode = stat.st_mode & 0o777;

    // open the destination file
    let to_c = CString::new(to_path.as_os_str().as_bytes()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "to",
            "path contains nul byte",
        ))
        .boxed()
    })?;
    let dst_fd = unsafe {
        libc::open(
            to_c.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            mode as libc::c_uint,
        )
    };
    if dst_fd < 0 {
        return Err(core_platform::io_error(
            "open",
            Some(to_path.to_string_lossy().as_ref()),
        ));
    }
    let _dst_guard = FdGuard(dst_fd);

    // copy the file contents in chunks
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let rc = unsafe {
            libc::read(
                src_fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
            )
        };
        if rc == 0 {
            break;
        }
        if rc < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }
            return Err(core_platform::io_error(
                "read",
                Some(from_path.to_string_lossy().as_ref()),
            ));
        }

        let mut written = 0;
        let total = rc as usize;
        while written < total {
            let slice = &buffer[written..total];
            let wc =
                unsafe { libc::write(dst_fd, slice.as_ptr() as *const libc::c_void, slice.len()) };
            if wc < 0 {
                let errno = core_platform::get_errno();
                if errno == libc::EINTR {
                    continue;
                }
                return Err(core_platform::io_error(
                    "write",
                    Some(to_path.to_string_lossy().as_ref()),
                ));
            }
            written += wc as usize;
        }
    }

    Ok(())
}

/// Binding implementation for `destack.fs.copyfileUtf16`.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // reject unsupported flags
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported copyfile flags",
        ))
        .boxed());
    }

    // report unsupported copyfile calls on non-windows platforms
    {
        let _ = (from, to, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfileUtf16")).boxed())
    }

    // perform the file copy on windows platforms
}

/// Binding implementation for `destack.fs.fchmod`.
pub(crate) unsafe fn destack_fs_fchmod(
    context: &RuntimeCallContext,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    {
        let fd = file_descriptor(context, handle)?;
        let rc = unsafe { libc::fchmod(fd, mode.0 as libc::mode_t) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("fchmod", None))
    }
}

/// Binding implementation for `destack.fs.fchown`.
pub(crate) unsafe fn destack_fs_fchown(
    context: &RuntimeCallContext,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    {
        let fd = file_descriptor(context, handle)?;
        let rc = unsafe { libc::fchown(fd, uid, gid) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("fchown", None))
    }
}

/// Binding implementation for `destack.fs.fdatasync`.
pub(crate) unsafe fn destack_fs_fdatasync(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    {
        let fd = file_descriptor(context, handle)?;
        let rc = os::fdatasync(fd);
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("fdatasync", None))
    }
}

/// Binding implementation for `destack.fs.fstat`.
pub(crate) unsafe fn destack_fs_fstat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file descriptor
    {
        let fd = file_descriptor(context, handle)?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::fstat(fd, &mut stat) };
        if rc != 0 {
            return Err(core_platform::io_error("fstat", None));
        }

        unsafe {
            *out = stat_from_libc(stat);
        }

        Ok(())
    }
}

/// Binding implementation for `destack.fs.fstatfs`.
pub(crate) unsafe fn destack_fs_fstatfs(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file descriptor
    {
        let fd = file_descriptor(context, handle)?;
        let statfs = os::statfs_for_fd(fd)?;
        unsafe {
            *out = statfs;
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.fsync`.
pub(crate) unsafe fn destack_fs_fsync(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    {
        let fd = file_descriptor(context, handle)?;
        let rc = unsafe { libc::fsync(fd) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("fsync", None))
    }
}

/// Binding implementation for `destack.fs.ftruncate`.
pub(crate) unsafe fn destack_fs_ftruncate(
    context: &RuntimeCallContext,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    {
        let fd = file_descriptor(context, handle)?;
        let offset = offset_to_off_t(size)?;
        let rc = unsafe { libc::ftruncate(fd, offset) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("ftruncate", None))
    }
}

/// Binding implementation for `destack.fs.seek`.
pub(crate) unsafe fn destack_fs_seek(
    context: &RuntimeCallContext,
    out: *mut FileOffset,
    handle: FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file descriptor
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let whence = match whence {
        SeekWhence::Set => libc::SEEK_SET,
        SeekWhence::Cur => libc::SEEK_CUR,
        SeekWhence::End => libc::SEEK_END,
    };

    // seek and return the new position
    let rc = unsafe { libc::lseek(fd, offset, whence) };
    if rc < 0 {
        return Err(core_platform::io_error("lseek", None));
    }

    unsafe {
        *out = FileOffset(rc as i64);
    }

    Ok(())
}

/// Binding implementation for `destack.fs.fadvise`.
pub(crate) unsafe fn destack_fs_fadvise(
    context: &RuntimeCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let length = length.0 as libc::off_t;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let advice = match advice {
            FileAdvice::Normal => libc::POSIX_FADV_NORMAL,
            FileAdvice::Sequential => libc::POSIX_FADV_SEQUENTIAL,
            FileAdvice::Random => libc::POSIX_FADV_RANDOM,
            FileAdvice::WillNeed => libc::POSIX_FADV_WILLNEED,
            FileAdvice::DontNeed => libc::POSIX_FADV_DONTNEED,
            FileAdvice::NoReuse => libc::POSIX_FADV_NOREUSE,
        };

        // advise the kernel
        let rc = unsafe { libc::posix_fadvise(fd, offset, length, advice) };
        if rc != 0 {
            return Err(core_platform::io_error_with_errno(
                "posix_fadvise",
                rc,
                None,
            ));
        }

        return Ok(());
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let _ = (offset, length);
        let enable_readahead = matches!(advice, FileAdvice::Sequential | FileAdvice::WillNeed);
        let rc = unsafe { libc::fcntl(fd, libc::F_RDAHEAD, enable_readahead as libc::c_int) };
        if rc == -1 {
            return Err(core_platform::io_error("fcntl(F_RDAHEAD)", None));
        }

        Ok(())
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        let _ = (fd, offset, length, advice);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fadvise")).boxed())
    }
}

/// Binding implementation for `destack.fs.fallocate`.
pub(crate) unsafe fn destack_fs_fallocate(
    context: &RuntimeCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let length = length.0 as libc::off_t;

    // attempt to allocate space using the best available syscall
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let rc = unsafe { libc::fallocate(fd, flags.0 as libc::c_int, offset, length) };
        if rc != 0 {
            return Err(core_platform::io_error("fallocate", None));
        }
        return Ok(());
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if flags.0 != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.fs.fallocate flags",
            ))
            .boxed());
        }

        let mut store = libc::fstore_t {
            fst_flags: libc::F_ALLOCATECONTIG,
            fst_posmode: libc::F_PEOFPOSMODE,
            fst_offset: offset,
            fst_length: length,
            fst_bytesalloc: 0,
        };
        let mut rc = unsafe { libc::fcntl(fd, libc::F_PREALLOCATE, &mut store) };
        if rc == -1 {
            store.fst_flags = libc::F_ALLOCATEALL;
            rc = unsafe { libc::fcntl(fd, libc::F_PREALLOCATE, &mut store) };
        }
        if rc == -1 {
            return Err(core_platform::io_error("fcntl(F_PREALLOCATE)", None));
        }

        let end = offset.checked_add(length).ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value("length", "overflow")).boxed()
        })?;
        let rc = unsafe { libc::ftruncate(fd, end) };
        if rc != 0 {
            return Err(core_platform::io_error("ftruncate", None));
        }
        Ok(())
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        let _ = (fd, offset, length, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fallocate")).boxed())
    }
}

/// Binding implementation for `destack.fs.syncFileRange`.
pub(crate) unsafe fn destack_fs_sync_file_range(
    context: &RuntimeCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let length = length.0 as libc::off_t;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let rc = unsafe { libc::sync_file_range(fd, offset, length, flags.0 as libc::c_uint) };
        if rc != 0 {
            return Err(core_platform::io_error("sync_file_range", None));
        }
        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (offset, length, flags);
        let rc = unsafe { libc::fsync(fd) };
        if rc != 0 {
            return Err(core_platform::io_error("fsync", None));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.dup`.
pub(crate) unsafe fn destack_fs_dup(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // duplicate the file descriptor
    let fd = file_descriptor(context, handle)?;
    let dup_fd = unsafe { libc::dup(fd) };
    if dup_fd < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    // register the new handle
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_fd(dup_fd)
        .with_finalizer(FdFinalizer { fd: dup_fd });
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Binding implementation for `destack.fs.dup2`.
pub(crate) unsafe fn destack_fs_dup2(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // duplicate the file descriptor
    let fd = file_descriptor(context, handle)?;
    let dup_fd = unsafe { libc::dup(fd) };
    if dup_fd < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    // replace the target resource entry
    if let Some(entry) = context.runtime().resources.remove(target.0) {
        entry.finalize(target.0);
    }
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_fd(dup_fd)
        .with_finalizer(FdFinalizer { fd: dup_fd });
    context.runtime().resources.insert_with_id(target.0, entry);
    unsafe {
        *out = target;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.dup3`.
pub(crate) unsafe fn destack_fs_dup3(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
    flags: OpenFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // duplicate the file descriptor
    let fd = file_descriptor(context, handle)?;
    let dup_fd = unsafe { libc::dup(fd) };
    if dup_fd < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    // apply CLOEXEC when requested
    if flags.0 & libc::O_CLOEXEC as u32 != 0 {
        let rc = unsafe { libc::fcntl(dup_fd, libc::F_SETFD, libc::FD_CLOEXEC) };
        if rc != 0 {
            unsafe {
                libc::close(dup_fd);
            }
            return Err(core_platform::io_error("fcntl", None));
        }
    }

    // replace the target resource entry
    if let Some(entry) = context.runtime().resources.remove(target.0) {
        entry.finalize(target.0);
    }
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_fd(dup_fd)
        .with_finalizer(FdFinalizer { fd: dup_fd });
    context.runtime().resources.insert_with_id(target.0, entry);
    unsafe {
        *out = target;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.futimes`.
pub(crate) unsafe fn destack_fs_futimes(
    context: &RuntimeCallContext,
    handle: FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    {
        let fd = file_descriptor(context, handle)?;
        let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
        let rc = unsafe { libc::futimens(fd, times.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("futimens", None))
    }
}

/// Binding implementation for `destack.fs.link`.
/// Binding implementation for `destack.fs.linkBytes`.
pub(crate) unsafe fn destack_fs_link_bytes(
    context: &RuntimeCallContext,
    existingpath: PathBytes,
    newpath: PathBytes,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // create the link on unix platforms
    {
        let existing = resolve_path_bytes_cstring(existingpath, "existingPath")?;
        let newpath = resolve_path_bytes_cstring(newpath, "newPath")?;
        let rc = unsafe { libc::link(existing.as_ptr(), newpath.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("link", None))
    }
}

/// Binding implementation for `destack.fs.linkUtf16`.
pub(crate) unsafe fn destack_fs_link_utf16(
    context: &RuntimeCallContext,
    existingpath: PathUtf16,
    newpath: PathUtf16,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported link calls on non-windows platforms
    {
        let _ = (existingpath, newpath);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkUtf16")).boxed())
    }

    // create the link on windows platforms
}

/// Binding implementation for `destack.fs.lstat`.
/// Binding implementation for `destack.fs.lstatBytes`.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stat data without following symlinks
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::lstat(c_path.as_ptr(), &mut stat) };
        if rc != 0 {
            return Err(core_platform::io_error("lstat", None));
        }

        unsafe {
            *out = stat_from_libc(stat);
        }

        Ok(())
    }
}

/// Binding implementation for `destack.fs.lstatUtf16`.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported lstat calls on non-windows platforms
    {
        let _ = (path, out);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstatUtf16")).boxed())
    }

    // read the stat data on windows platforms
}

/// Binding implementation for `destack.fs.lutimes`.
/// Binding implementation for `destack.fs.lutimesBytes`.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // apply timestamps on unix platforms without following symlinks
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
        let rc = unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                c_path.as_ptr(),
                times.as_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("utimensat", None))
    }
}

/// Binding implementation for `destack.fs.lutimesUtf16`.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported lutimes calls on non-windows platforms
    {
        let _ = (path, atimens, mtimens);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesUtf16")).boxed())
    }

    // apply timestamps on windows platforms
}

/// Binding implementation for `destack.fs.utimensat`.
/// Binding implementation for `destack.fs.utimensatBytes`.
pub(crate) unsafe fn destack_fs_utimensat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // update timestamps relative to the directory on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
        let rc = unsafe {
            libc::utimensat(
                resource.fd,
                c_path.as_ptr(),
                times.as_ptr(),
                flags.0 as libc::c_int,
            )
        };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("utimensat", None))
    }

    // report unsupported utimensat calls on non-unix platforms
}

/// Binding implementation for `destack.fs.utimensatUtf16`.
pub(crate) unsafe fn destack_fs_utimensat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported utimensat calls on non-windows platforms
    {
        let _ = (dir, path, atimens, mtimens, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimensatUtf16")).boxed())
    }

    // apply timestamps on windows platforms
}

/// Binding implementation for `destack.fs.mkdir`.
/// Binding implementation for `destack.fs.mkdtemp`.
/// Binding implementation for `destack.fs.mkdtempBytes`.
pub(crate) unsafe fn destack_fs_mkdtemp_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    template: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // create the temporary directory on unix platforms
    {
        let bytes = unsafe { template.0.as_slice()? };
        if bytes.contains(&0) {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "template",
                "template contains nul byte",
            ))
            .boxed());
        }
        let mut buffer = bytes.to_vec();
        buffer.push(0);
        let ptr = buffer.as_mut_ptr() as *mut libc::c_char;
        let result = unsafe { libc::mkdtemp(ptr) };
        if result.is_null() {
            return Err(core_platform::io_error("mkdtemp", None));
        }

        let value = unsafe { CStr::from_ptr(result) }.to_bytes().to_vec();
        unsafe {
            *out = PathBytesAbi::<NativeAbi>(context.store_array(value));
        }

        Ok(())
    }
}

/// Binding implementation for `destack.fs.mkdtempUtf16`.
#[allow(dead_code)]
pub(crate) unsafe fn destack_fs_mkdtemp_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    template: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported mkdtemp calls on non-windows platforms
    {
        let _ = (context, template, out);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdtempUtf16")).boxed())
    }

    // create the temporary directory on windows platforms
}

/// Binding implementation for `destack.fs.open`.
/// Binding implementation for `destack.fs.openBytes`.
pub(crate) unsafe fn destack_fs_open_bytes(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the file on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let open_flags = flags.0 as libc::c_int | libc::O_CLOEXEC;
        let fd = unsafe { libc::open(c_path.as_ptr(), open_flags, mode.0 as libc::c_uint) };
        if fd < 0 {
            return Err(core_platform::io_error("open", None));
        }

        let entry = ResourceEntry::new(ResourceKind::File)
            .with_fd(fd)
            .with_finalizer(FdFinalizer { fd });
        let resource_id = context.runtime().resources.insert(entry);
        unsafe {
            *out = FileHandle(resource_id);
        }

        Ok(())
    }
}

/// Binding implementation for `destack.fs.openUtf16`.
pub(crate) unsafe fn destack_fs_open_utf16(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported open calls on non-windows platforms
    {
        let _ = (context, path, flags, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openUtf16")).boxed())
    }

    // open the file on windows platforms
}

/// Binding implementation for `destack.fs.opendir`.
/// Binding implementation for `destack.fs.opendirBytes`.
pub(crate) unsafe fn destack_fs_opendir_bytes(
    context: &RuntimeCallContext,
    out: *mut DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the directory on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC;
        let fd = unsafe { libc::open(c_path.as_ptr(), flags) };
        if fd < 0 {
            return Err(core_platform::io_error("opendir", None));
        }

        let path_buf = resolve_path_bytes(path, "path")?;
        let resource = DirectoryResource { path: path_buf, fd };
        let entry = ResourceEntry::new(ResourceKind::Directory)
            .with_payload(resource)
            .with_finalizer(FdFinalizer { fd });
        let resource_id = context.runtime().resources.insert(entry);
        unsafe {
            *out = DirectoryHandle(resource_id);
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.opendirUtf16`.
pub(crate) unsafe fn destack_fs_opendir_utf16(
    context: &RuntimeCallContext,
    out: *mut DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported opendir calls on non-windows platforms
    {
        let _ = (context, path, out);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.opendirUtf16")).boxed())
    }

    // open the directory on windows platforms
}

/// Binding implementation for `destack.fs.read`.
pub(crate) unsafe fn destack_fs_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the target buffer
    let buffer = unsafe { buffer.as_mut_slice()? };

    // read from the file on unix platforms
    let fd = file_descriptor(context, handle)?;
    let rc = unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("read", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.pread`.
pub(crate) unsafe fn destack_fs_pread(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the target buffer
    let buffer = unsafe { buffer.as_mut_slice()? };

    // read from the file on unix platforms
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let rc = unsafe {
        libc::pread(
            fd,
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
            offset,
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("pread", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.readdir`.
pub(crate) unsafe fn destack_fs_readdir(
    context: &RuntimeCallContext,
    out: *mut NativeArray<Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read directory entries on unix platforms
    {
        let resource = directory_resource(context, handle)?;
        let dup_fd = unsafe { libc::dup(resource.fd) };
        if dup_fd < 0 {
            return Err(core_platform::io_error("dup", None));
        }

        let dirp = unsafe { libc::fdopendir(dup_fd) };
        if dirp.is_null() {
            unsafe {
                libc::close(dup_fd);
            }
            return Err(core_platform::io_error("fdopendir", None));
        }

        struct DirGuard(*mut libc::DIR);
        impl Drop for DirGuard {
            fn drop(&mut self) {
                unsafe {
                    libc::closedir(self.0);
                }
            }
        }

        let _guard = DirGuard(dirp);
        let mut dirents = Vec::new();
        loop {
            core_platform::set_errno(0);
            let entry = unsafe { libc::readdir(dirp) };
            if entry.is_null() {
                if core_platform::get_errno() != 0 {
                    return Err(core_platform::io_error(
                        "readdir",
                        Some(resource.path.to_string_lossy().as_ref()),
                    ));
                }
                break;
            }

            let name_ptr = unsafe { (*entry).d_name.as_ptr() };
            let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
            let name_bytes = name.to_bytes();
            if name_bytes == b"." || name_bytes == b".." {
                continue;
            }
            let mut kind = dirent_kind_from_type(unsafe { (*entry).d_type });
            if kind == DirentKind::Unknown {
                let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
                let rc = unsafe {
                    libc::fstatat(
                        resource.fd,
                        name_ptr,
                        stat.as_mut_ptr(),
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                };
                if rc == 0 {
                    let stat = unsafe { stat.assume_init() };
                    kind = dirent_kind_from_mode(stat.st_mode);
                }
            }
            let name = PathBytesAbi::<NativeAbi>(context.store_array(name_bytes.to_vec()));
            dirents.push(Dirent {
                name: core_fs::path_ref_from_bytes(name),
                kind,
            });
        }

        let array = context.store_array(dirents);
        unsafe {
            *out = array;
        }

        Ok(())
    }
}

/// Binding implementation for `destack.fs.readlink`.
/// Binding implementation for `destack.fs.readlinkBytes`.
pub(crate) unsafe fn destack_fs_readlink_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the symlink target on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let bytes = readlink_bytes_at(libc::AT_FDCWD, &c_path)?;
        unsafe {
            *out = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.readlinkUtf16`.
#[allow(dead_code)]
pub(crate) unsafe fn destack_fs_readlink_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported readlink calls on non-windows platforms
    {
        let _ = (context, path);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkUtf16")).boxed())
    }

    // read the symlink target on windows platforms
}

/// Binding implementation for `destack.fs.readv`.
pub(crate) unsafe fn destack_fs_readv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // read from the file on unix platforms
    let fd = file_descriptor(context, handle)?;
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_mut_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }
    let rc = unsafe { libc::readv(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::io_error("readv", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.preadv`.
pub(crate) unsafe fn destack_fs_preadv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // read from the file on unix platforms
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_mut_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }
    let rc = unsafe { libc::preadv(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int, offset) };
    if rc < 0 {
        return Err(core_platform::io_error("preadv", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.realpath`.
/// Binding implementation for `destack.fs.realpathBytes`.
pub(crate) unsafe fn destack_fs_realpath_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the canonical path on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let resolved = unsafe { libc::realpath(c_path.as_ptr(), std::ptr::null_mut()) };
        if resolved.is_null() {
            return Err(core_platform::io_error("realpath", None));
        }

        let bytes = unsafe { CStr::from_ptr(resolved) }.to_bytes().to_vec();
        unsafe {
            libc::free(resolved as *mut libc::c_void);
        }
        unsafe {
            *out = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.realpathUtf16`.
#[allow(dead_code)]
pub(crate) unsafe fn destack_fs_realpath_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported realpath calls on non-windows platforms
    {
        let _ = (context, path);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.realpathUtf16")).boxed())
    }

    // resolve the canonical path on windows platforms
}

/// Binding implementation for `destack.fs.rename`.
/// Binding implementation for `destack.fs.renameBytes`.
pub(crate) unsafe fn destack_fs_rename_bytes(
    context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // rename the path on unix platforms
    {
        let from = resolve_path_bytes_cstring(from, "from")?;
        let to = resolve_path_bytes_cstring(to, "to")?;
        let result = unsafe { libc::rename(from.as_ptr(), to.as_ptr()) };
        if result != 0 {
            return Err(core_platform::io_error("rename", None));
        }

        Ok(())
    }
}

/// Binding implementation for `destack.fs.renameUtf16`.
pub(crate) unsafe fn destack_fs_rename_utf16(
    context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported rename calls on non-windows platforms
    {
        let _ = (from, to);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameUtf16")).boxed())
    }

    // rename the path on windows platforms
}

/// Binding implementation for `destack.fs.rmdir`.
/// Binding implementation for `destack.fs.rmdirBytes`.
pub(crate) unsafe fn destack_fs_rmdir_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // remove the directory on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::rmdir(c_path.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("rmdir", None))
    }
}

/// Binding implementation for `destack.fs.rmdirUtf16`.
pub(crate) unsafe fn destack_fs_rmdir_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported rmdir calls on non-windows platforms
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirUtf16")).boxed())
    }

    // remove the directory on windows platforms
}

/// Binding implementation for `destack.fs.stat`.
/// Binding implementation for `destack.fs.statBytes`.
pub(crate) unsafe fn destack_fs_stat_bytes(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stat data on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::stat(c_path.as_ptr(), &mut stat) };
        if rc != 0 {
            return Err(core_platform::io_error("stat", None));
        }

        unsafe {
            *out = stat_from_libc(stat);
        }

        Ok(())
    }
}

/// Binding implementation for `destack.fs.statUtf16`.
pub(crate) unsafe fn destack_fs_stat_utf16(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported stat calls on non-windows platforms
    {
        let _ = (path, out);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statUtf16")).boxed())
    }

    // read the stat data on windows platforms
}

/// Binding implementation for `destack.fs.statfs`.
/// Binding implementation for `destack.fs.statfsBytes`.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the statfs data on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let statfs = os::statfs_for_path(&c_path)?;
        unsafe {
            *out = statfs;
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.statfsUtf16`.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported statfs calls on non-windows platforms
    {
        let _ = (path, out);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statfsUtf16")).boxed())
    }

    // read the statfs data on windows platforms
}

/// Binding implementation for `destack.fs.symlink`.
/// Binding implementation for `destack.fs.symlinkBytes`.
pub(crate) unsafe fn destack_fs_symlink_bytes(
    context: &RuntimeCallContext,
    target: PathBytes,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // ignore unused kind
    let _ = kind;

    // create the symlink on unix platforms
    {
        let target = resolve_path_bytes_cstring(target, "target")?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::symlink(target.as_ptr(), path.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("symlink", None))
    }
}

/// Binding implementation for `destack.fs.symlinkUtf16`.
pub(crate) unsafe fn destack_fs_symlink_utf16(
    context: &RuntimeCallContext,
    target: PathUtf16,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // ignore unused kind
    let _ = kind;

    // report unsupported symlink calls on non-windows platforms
    {
        let _ = (target, path);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkUtf16")).boxed())
    }

    // create the symlink on windows platforms
}

/// Binding implementation for `destack.fs.truncate`.
/// Binding implementation for `destack.fs.truncateBytes`.
pub(crate) unsafe fn destack_fs_truncate_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // truncate the file on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let offset = offset_to_off_t(size)?;
        let rc = unsafe { libc::truncate(c_path.as_ptr(), offset) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("truncate", None))
    }
}

/// Binding implementation for `destack.fs.truncateUtf16`.
pub(crate) unsafe fn destack_fs_truncate_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported truncate calls on non-windows platforms
    {
        let _ = (path, size);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateUtf16")).boxed())
    }

    // truncate the file on windows platforms
}

/// Binding implementation for `destack.fs.unlink`.
/// Binding implementation for `destack.fs.unlinkBytes`.
pub(crate) unsafe fn destack_fs_unlink_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // unlink the file on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::unlink(c_path.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("unlink", None))
    }
}

/// Binding implementation for `destack.fs.unlinkUtf16`.
pub(crate) unsafe fn destack_fs_unlink_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported unlink calls on non-windows platforms
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkUtf16")).boxed())
    }

    // unlink the file on windows platforms
}

/// Binding implementation for `destack.fs.utimes`.
/// Binding implementation for `destack.fs.utimesBytes`.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // apply timestamps on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
        let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c_path.as_ptr(), times.as_ptr(), 0) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("utimensat", None))
    }
}

/// Binding implementation for `destack.fs.utimesUtf16`.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported utimes calls on non-windows platforms
    {
        let _ = (path, atimens, mtimens);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesUtf16")).boxed())
    }

    // apply timestamps on windows platforms
}

/// Binding implementation for `destack.fs.write`.
pub(crate) unsafe fn destack_fs_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the source buffer
    let buffer = unsafe { buffer.as_slice()? };

    // write to the file on unix platforms
    let fd = file_descriptor(context, handle)?;
    let rc = unsafe { libc::write(fd, buffer.as_ptr() as *const libc::c_void, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("write", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.pwrite`.
pub(crate) unsafe fn destack_fs_pwrite(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the source buffer
    let buffer = unsafe { buffer.as_slice()? };

    // write to the file on unix platforms
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let rc = unsafe {
        libc::pwrite(
            fd,
            buffer.as_ptr() as *const libc::c_void,
            buffer.len(),
            offset,
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("pwrite", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.writev`.
pub(crate) unsafe fn destack_fs_writev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // write to the file on unix platforms
    let fd = file_descriptor(context, handle)?;
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }
    let rc = unsafe { libc::writev(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::io_error("writev", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.pwritev`.
pub(crate) unsafe fn destack_fs_pwritev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // write to the file on unix platforms
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }
    let rc = unsafe { libc::pwritev(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int, offset) };
    if rc < 0 {
        return Err(core_platform::io_error("pwritev", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.openat`.
/// Binding implementation for `destack.fs.openatBytes`.
pub(crate) unsafe fn destack_fs_openat_bytes(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the file on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let open_flags = flags.0 as libc::c_int | libc::O_CLOEXEC;
        let fd = unsafe {
            libc::openat(
                resource.fd,
                path.as_ptr(),
                open_flags,
                mode.0 as libc::c_uint,
            )
        };
        if fd < 0 {
            return Err(core_platform::io_error("openat", None));
        }
        let entry = ResourceEntry::new(ResourceKind::File)
            .with_fd(fd)
            .with_finalizer(FdFinalizer { fd });
        let handle = context.runtime().resources.insert(entry);
        unsafe {
            *out = FileHandle(handle);
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.openatUtf16`.
pub(crate) unsafe fn destack_fs_openat_utf16(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported openat calls on non-windows platforms
    {
        let _ = (context, out, dir, path, flags, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openatUtf16")).boxed())
    }

    // open the file on windows platforms
}

/// Binding implementation for `destack.fs.openat2`.
/// Binding implementation for `destack.fs.openat2Bytes`.
pub(crate) unsafe fn destack_fs_openat2_bytes(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    how: OpenOptions,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the file using openat2 on linux platforms
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let resource = directory_resource(context, dir)?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let mut open_how: libc::open_how = unsafe { std::mem::zeroed() };
        open_how.flags = how.flags.0 as u64;
        open_how.mode = how.mode.0 as u64;
        open_how.resolve = how.resolve.0 as u64;
        let fd = unsafe {
            libc::syscall(
                libc::SYS_openat2,
                resource.fd,
                path.as_ptr(),
                &open_how as *const libc::open_how,
                std::mem::size_of::<libc::open_how>(),
            )
        } as libc::c_int;
        if fd < 0 {
            return Err(core_platform::io_error("openat2", None));
        }
        let entry = ResourceEntry::new(ResourceKind::File)
            .with_fd(fd)
            .with_finalizer(FdFinalizer { fd });
        let handle = context.runtime().resources.insert(entry);
        unsafe {
            *out = FileHandle(handle);
        }
        return Ok(());
    }

    // fall back to openat when resolve flags are empty on other unix platforms
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        if how.resolve.0 != 0 {
            return Err(
                RuntimeError::from(PlatformError::not_supported("destack.fs.openat2")).boxed(),
            );
        }
        unsafe { destack_fs_openat_bytes(context, out, dir, path, how.flags, how.mode) }
    }
}

/// Binding implementation for `destack.fs.openat2Utf16`.
pub(crate) unsafe fn destack_fs_openat2_utf16(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    how: OpenOptions,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported openat2 calls on non-windows platforms
    {
        let _ = (context, out, dir, path, how);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openat2Utf16")).boxed())
    }

    // open the file on windows platforms
}

/// Binding implementation for `destack.fs.mkdirBytes`.
pub(crate) unsafe fn destack_fs_mkdir_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // create the directory on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::mkdir(c_path.as_ptr(), mode.0 as libc::mode_t) };
        if rc == 0 {
            return Ok(());
        }

        Err(core_platform::io_error("mkdir", None))
    }
}

/// Binding implementation for `destack.fs.mkdirUtf16`.
pub(crate) unsafe fn destack_fs_mkdir_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ignore unused context
    let _ = context;

    // report unsupported mkdir calls on non-windows platforms
    {
        let _ = (path, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdirUtf16")).boxed())
    }

    // create the directory on windows platforms
}

/// Binding implementation for `destack.fs.mkdirat`.
/// Binding implementation for `destack.fs.mkdiratBytes`.
pub(crate) unsafe fn destack_fs_mkdirat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    // create the directory on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let result = unsafe { libc::mkdirat(resource.fd, path.as_ptr(), mode.0 as libc::mode_t) };
        if result != 0 {
            return Err(core_platform::io_error("mkdirat", None));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.mkdiratUtf16`.
pub(crate) unsafe fn destack_fs_mkdirat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    // report unsupported mkdirat calls on non-windows platforms
    {
        let _ = (context, dir, path, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdiratUtf16")).boxed())
    }

    // create the directory on windows platforms
}

/// Binding implementation for `destack.fs.renameat`.
/// Binding implementation for `destack.fs.renameatBytes`.
pub(crate) unsafe fn destack_fs_renameat_bytes(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
) -> RuntimeResult<()> {
    // rename the path on unix platforms
    {
        let from_resource = directory_resource(context, from_dir)?;
        let to_resource = directory_resource(context, to_dir)?;
        let from = resolve_path_bytes_cstring(from, "from")?;
        let to = resolve_path_bytes_cstring(to, "to")?;
        let result =
            unsafe { libc::renameat(from_resource.fd, from.as_ptr(), to_resource.fd, to.as_ptr()) };
        if result != 0 {
            return Err(core_platform::io_error("renameat", None));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.renameatUtf16`.
pub(crate) unsafe fn destack_fs_renameat_utf16(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
) -> RuntimeResult<()> {
    // report unsupported renameat calls on non-windows platforms
    {
        let _ = (context, from_dir, from, to_dir, to);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameatUtf16")).boxed())
    }

    // rename the path on windows platforms
}

/// Binding implementation for `destack.fs.renameat2`.
/// Binding implementation for `destack.fs.renameat2Bytes`.
pub(crate) unsafe fn destack_fs_renameat2_bytes(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    // rename the path with renameat2 on linux platforms
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let from_resource = directory_resource(context, from_dir)?;
        let to_resource = directory_resource(context, to_dir)?;
        let from = resolve_path_bytes_cstring(from, "from")?;
        let to = resolve_path_bytes_cstring(to, "to")?;
        let rc = unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                from_resource.fd,
                from.as_ptr(),
                to_resource.fd,
                to.as_ptr(),
                flags.0 as libc::c_uint,
            )
        } as libc::c_int;
        if rc != 0 {
            return Err(core_platform::io_error("renameat2", None));
        }
        return Ok(());
    }

    // fall back to renameat when no flags are requested on other unix platforms
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        if flags.0 != 0 {
            return Err(
                RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2")).boxed(),
            );
        }
        unsafe { destack_fs_renameat_bytes(context, from_dir, from, to_dir, to) }
    }
}

/// Binding implementation for `destack.fs.renameat2Utf16`.
pub(crate) unsafe fn destack_fs_renameat2_utf16(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    // report unsupported renameat2 calls on non-windows platforms
    {
        let _ = (context, from_dir, from, to_dir, to, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2Utf16")).boxed())
    }

    // rename the path on windows platforms
}

/// Binding implementation for `destack.fs.unlinkat`.
/// Binding implementation for `destack.fs.unlinkatBytes`.
pub(crate) unsafe fn destack_fs_unlinkat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // unlink the path on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let result = unsafe { libc::unlinkat(resource.fd, path.as_ptr(), flags.0 as libc::c_int) };
        if result != 0 {
            return Err(core_platform::io_error("unlinkat", None));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.unlinkatUtf16`.
pub(crate) unsafe fn destack_fs_unlinkat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // report unsupported unlinkat calls on non-windows platforms
    {
        let _ = (context, dir, path, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkatUtf16")).boxed())
    }

    // unlink the path on windows platforms
}

/// Binding implementation for `destack.fs.linkat`.
/// Binding implementation for `destack.fs.linkatBytes`.
pub(crate) unsafe fn destack_fs_linkat_bytes(
    context: &RuntimeCallContext,
    existing_dir: DirectoryHandle,
    existing_path: PathBytes,
    new_dir: DirectoryHandle,
    new_path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // link the paths on unix platforms
    {
        let existing_resource = directory_resource(context, existing_dir)?;
        let new_resource = directory_resource(context, new_dir)?;
        let existing_path = resolve_path_bytes_cstring(existing_path, "existingPath")?;
        let new_path = resolve_path_bytes_cstring(new_path, "newPath")?;
        let result = unsafe {
            libc::linkat(
                existing_resource.fd,
                existing_path.as_ptr(),
                new_resource.fd,
                new_path.as_ptr(),
                flags.0 as libc::c_int,
            )
        };
        if result != 0 {
            return Err(core_platform::io_error("linkat", None));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.linkatUtf16`.
pub(crate) unsafe fn destack_fs_linkat_utf16(
    context: &RuntimeCallContext,
    existing_dir: DirectoryHandle,
    existing_path: PathUtf16,
    new_dir: DirectoryHandle,
    new_path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // report unsupported linkat calls on non-windows platforms
    {
        let _ = (
            context,
            existing_dir,
            existing_path,
            new_dir,
            new_path,
            flags,
        );
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkatUtf16")).boxed())
    }

    // link the paths on windows platforms
}

/// Binding implementation for `destack.fs.symlinkat`.
/// Binding implementation for `destack.fs.symlinkatBytes`.
pub(crate) unsafe fn destack_fs_symlinkat_bytes(
    context: &RuntimeCallContext,
    target: PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // ignore unused kind
    let _ = kind;

    // create the symlink on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let target = resolve_path_bytes_cstring(target, "target")?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let result = unsafe { libc::symlinkat(target.as_ptr(), resource.fd, path.as_ptr()) };
        if result != 0 {
            return Err(core_platform::io_error("symlinkat", None));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.symlinkatUtf16`.
pub(crate) unsafe fn destack_fs_symlinkat_utf16(
    context: &RuntimeCallContext,
    target: PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    // report unsupported symlinkat calls on non-windows platforms
    {
        let _ = (context, target, dir, path, kind);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkatUtf16")).boxed())
    }

    // create the symlink on windows platforms
}

/// Binding implementation for `destack.fs.readlinkat`.
/// Binding implementation for `destack.fs.readlinkatBytes`.
pub(crate) unsafe fn destack_fs_readlinkat_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the symlink target on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let bytes = readlink_bytes_at(resource.fd, &path)?;
        unsafe {
            *out = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.readlinkatUtf16`.
#[allow(dead_code)]
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported readlinkat calls on non-windows platforms
    {
        let _ = (context, dir, path);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkatUtf16")).boxed())
    }

    // read the symlink target on windows platforms
}

/// Binding implementation for `destack.fs.statat`.
/// Binding implementation for `destack.fs.statatBytes`.
pub(crate) unsafe fn destack_fs_statat_bytes(
    context: &RuntimeCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // stat the path on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                resource.fd,
                path.as_ptr(),
                stat.as_mut_ptr(),
                flags.0 as libc::c_int,
            )
        };
        if result != 0 {
            return Err(core_platform::io_error("statat", None));
        }
        let stat = unsafe { stat.assume_init() };
        unsafe {
            *out = stat_from_libc(stat);
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.statatUtf16`.
pub(crate) unsafe fn destack_fs_statat_utf16(
    context: &RuntimeCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported statat calls on non-windows platforms
    {
        let _ = (context, dir, path, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statatUtf16")).boxed())
    }

    // stat the path on windows platforms
}

/// Binding implementation for `destack.fs.lock`.
pub(crate) unsafe fn destack_fs_lock(
    context: &RuntimeCallContext,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    // lock the file on unix platforms
    {
        let fd = file_descriptor(context, handle)?;
        let result = unsafe { libc::flock(fd, flags.0 as libc::c_int) };
        if result != 0 {
            return Err(core_platform::io_error("flock", None));
        }
        Ok(())
    }
}

/// Binding implementation for `destack.fs.copyFileRange`.
pub(crate) unsafe fn destack_fs_copy_file_range(
    context: &RuntimeCallContext,
    out: *mut u64,
    src: FileHandle,
    src_offset: FileOffset,
    dst: FileHandle,
    dst_offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve file descriptors
    let src_fd = file_descriptor(context, src)?;
    let dst_fd = file_descriptor(context, dst)?;
    let mut src_offset = offset_to_off_t(src_offset)?;
    let mut dst_offset = offset_to_off_t(dst_offset)?;

    // copy the requested range in chunks
    let mut remaining = length.0;
    let mut total = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    while remaining > 0 {
        let chunk = remaining.min(buffer.len() as u64) as usize;
        let rc = unsafe {
            libc::pread(
                src_fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                chunk,
                src_offset,
            )
        };
        if rc < 0 {
            return Err(core_platform::io_error("pread", None));
        }
        if rc == 0 {
            break;
        }

        let bytes = rc as usize;
        let wc = unsafe {
            libc::pwrite(
                dst_fd,
                buffer.as_ptr() as *const libc::c_void,
                bytes,
                dst_offset,
            )
        };
        if wc < 0 {
            return Err(core_platform::io_error("pwrite", None));
        }

        let written = wc as u64;
        src_offset += written as libc::off_t;
        dst_offset += written as libc::off_t;
        total += written;
        remaining = remaining.saturating_sub(written);
        if written as usize != bytes {
            break;
        }
    }

    unsafe {
        *out = total;
    }

    Ok(())
}

/// Binding implementation for `destack.fs.sendfile`.
pub(crate) unsafe fn destack_fs_sendfile(
    context: &RuntimeCallContext,
    out: *mut u64,
    socket: SocketHandle,
    file: FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve file and socket descriptors
    let fd = file_descriptor(context, file)?;
    let sock_fd = platform_net::core::require_resource(
        context,
        socket.0,
        ResourceKind::Socket,
        "socket",
        |entry| {
            entry.fd().ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "socket handle missing descriptor",
                ))
                .boxed()
            })
        },
    )?;

    let total = {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            let mut offset = offset_to_off_t(offset)?;
            let rc = unsafe {
                libc::sendfile(
                    sock_fd,
                    fd,
                    &mut offset as *mut libc::off_t,
                    length.0 as libc::size_t,
                )
            };
            if rc < 0 {
                return Err(core_platform::io_error("sendfile", None));
            }
            rc as u64
        }

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let mut len = length.0 as libc::off_t;
            let rc = unsafe {
                libc::sendfile(
                    fd,
                    sock_fd,
                    offset.0 as libc::off_t,
                    &mut len,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if rc != 0 {
                return Err(core_platform::io_error("sendfile", None));
            }
            len as u64
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios"
        )))]
        {
            let mut remaining = length.0;
            let mut total = 0u64;
            let mut file_offset = offset_to_off_t(offset)?;
            let mut buffer = vec![0u8; 1024 * 1024];

            while remaining > 0 {
                let chunk = remaining.min(buffer.len() as u64) as usize;
                let read = unsafe {
                    libc::pread(
                        fd,
                        buffer.as_mut_ptr() as *mut libc::c_void,
                        chunk,
                        file_offset,
                    )
                };
                if read < 0 {
                    return Err(core_platform::io_error("pread", None));
                }
                if read == 0 {
                    break;
                }

                let mut sent = 0usize;
                let read = read as usize;
                let flags = os::send_flags();
                while sent < read {
                    let write = unsafe {
                        libc::send(
                            sock_fd,
                            buffer.as_ptr().add(sent) as *const libc::c_void,
                            read - sent,
                            flags,
                        )
                    };
                    if write < 0 {
                        return Err(core_platform::io_error("send", None));
                    }
                    sent += write as usize;
                }

                total = total.saturating_add(sent as u64);
                file_offset += sent as libc::off_t;
                remaining = remaining.saturating_sub(sent as u64);
            }

            total
        }
    };

    unsafe {
        *out = total;
    }

    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn getxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::getxattr(path, name, value, size, 0, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn getxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::getxattr(path, name, value, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn lgetxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::getxattr(path, name, value, size, 0, libc::XATTR_NOFOLLOW) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn lgetxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::lgetxattr(path, name, value, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn fgetxattr_fd(
    fd: libc::c_int,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::fgetxattr(fd, name, value, size, 0, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn fgetxattr_fd(
    fd: libc::c_int,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::fgetxattr(fd, name, value, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn listxattr_path(
    path: *const libc::c_char,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::listxattr(path, list, size, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn listxattr_path(
    path: *const libc::c_char,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::listxattr(path, list, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn llistxattr_path(
    path: *const libc::c_char,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::listxattr(path, list, size, libc::XATTR_NOFOLLOW) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn llistxattr_path(
    path: *const libc::c_char,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::llistxattr(path, list, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn flistxattr_fd(fd: libc::c_int, list: *mut libc::c_char, size: usize) -> libc::ssize_t {
    unsafe { libc::flistxattr(fd, list, size, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn flistxattr_fd(fd: libc::c_int, list: *mut libc::c_char, size: usize) -> libc::ssize_t {
    unsafe { libc::flistxattr(fd, list, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn setxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::setxattr(path, name, value, size, 0, flags) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn setxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::setxattr(path, name, value, size, flags) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn lsetxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::setxattr(path, name, value, size, 0, flags | libc::XATTR_NOFOLLOW) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn lsetxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::lsetxattr(path, name, value, size, flags) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn fsetxattr_fd(
    fd: libc::c_int,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::fsetxattr(fd, name, value, size, 0, flags) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn fsetxattr_fd(
    fd: libc::c_int,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::fsetxattr(fd, name, value, size, flags) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn removexattr_path(path: *const libc::c_char, name: *const libc::c_char) -> libc::c_int {
    unsafe { libc::removexattr(path, name, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn removexattr_path(path: *const libc::c_char, name: *const libc::c_char) -> libc::c_int {
    unsafe { libc::removexattr(path, name) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn lremovexattr_path(path: *const libc::c_char, name: *const libc::c_char) -> libc::c_int {
    unsafe { libc::removexattr(path, name, libc::XATTR_NOFOLLOW) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn lremovexattr_path(path: *const libc::c_char, name: *const libc::c_char) -> libc::c_int {
    unsafe { libc::lremovexattr(path, name) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe fn fremovexattr_fd(fd: libc::c_int, name: *const libc::c_char) -> libc::c_int {
    unsafe { libc::fremovexattr(fd, name, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
unsafe fn fremovexattr_fd(fd: libc::c_int, name: *const libc::c_char) -> libc::c_int {
    unsafe { libc::fremovexattr(fd, name) }
}

fn decode_xattr_list(
    context: &RuntimeCallContext,
    buffer: Vec<u8>,
) -> RuntimeResult<NativeArray<NativeStringRef>> {
    // split on nul separators
    let mut names = Vec::new();
    for entry in buffer.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let name = std::str::from_utf8(entry).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "xattr",
                "attribute name is not valid utf8",
            ))
            .boxed()
        })?;
        names.push(context.store_string(name));
    }

    Ok(context.store_array(names))
}

/// Binding implementation for `destack.fs.getxattrBytes`.
pub(crate) unsafe fn destack_fs_getxattr_bytes(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    path: PathBytes,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve path and name
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;

    // query the attribute size
    let size = unsafe { getxattr_path(path.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("getxattr", None));
    }

    // read the attribute payload
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe {
        getxattr_path(
            path.as_ptr(),
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("getxattr", None));
    }

    unsafe {
        *out = context.store_array(buffer);
    }

    Ok(())
}

/// Binding implementation for `destack.fs.getxattrUtf16`.
pub(crate) unsafe fn destack_fs_getxattr_utf16(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let size = unsafe { getxattr_path(path.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("getxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe {
        getxattr_path(
            path.as_ptr(),
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("getxattr", None));
    }
    unsafe {
        *out = context.store_array(buffer);
    }
    Ok(())
}

/// Binding implementation for `destack.fs.lgetxattrBytes`.
pub(crate) unsafe fn destack_fs_lgetxattr_bytes(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    path: PathBytes,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let size = unsafe { lgetxattr_path(path.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("lgetxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe {
        lgetxattr_path(
            path.as_ptr(),
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("lgetxattr", None));
    }
    unsafe {
        *out = context.store_array(buffer);
    }
    Ok(())
}

/// Binding implementation for `destack.fs.lgetxattrUtf16`.
pub(crate) unsafe fn destack_fs_lgetxattr_utf16(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let size = unsafe { lgetxattr_path(path.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("lgetxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe {
        lgetxattr_path(
            path.as_ptr(),
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("lgetxattr", None));
    }
    unsafe {
        *out = context.store_array(buffer);
    }
    Ok(())
}

/// Binding implementation for `destack.fs.fgetxattr`.
pub(crate) unsafe fn destack_fs_fgetxattr_handle(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let fd = file_descriptor(context, handle)?;
    let name = resolve_name_cstring(name, "name")?;
    let size = unsafe { fgetxattr_fd(fd, name.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("fgetxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe {
        fgetxattr_fd(
            fd,
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("fgetxattr", None));
    }
    unsafe {
        *out = context.store_array(buffer);
    }
    Ok(())
}

/// Binding implementation for `destack.fs.setxattrBytes`.
pub(crate) unsafe fn destack_fs_setxattr_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = context;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let value = unsafe { value.as_slice()? };
    let rc = unsafe {
        setxattr_path(
            path.as_ptr(),
            name.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            flags.0 as libc::c_int,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("setxattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.setxattrUtf16`.
pub(crate) unsafe fn destack_fs_setxattr_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = context;
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let value = unsafe { value.as_slice()? };
    let rc = unsafe {
        setxattr_path(
            path.as_ptr(),
            name.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            flags.0 as libc::c_int,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("setxattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.lsetxattrBytes`.
pub(crate) unsafe fn destack_fs_lsetxattr_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = context;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let value = unsafe { value.as_slice()? };
    let rc = unsafe {
        lsetxattr_path(
            path.as_ptr(),
            name.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            flags.0 as libc::c_int,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("lsetxattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.lsetxattrUtf16`.
pub(crate) unsafe fn destack_fs_lsetxattr_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = context;
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let value = unsafe { value.as_slice()? };
    let rc = unsafe {
        lsetxattr_path(
            path.as_ptr(),
            name.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            flags.0 as libc::c_int,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("lsetxattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.fsetxattr`.
pub(crate) unsafe fn destack_fs_fsetxattr_handle(
    context: &RuntimeCallContext,
    handle: FileHandle,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = context;
    let fd = file_descriptor(context, handle)?;
    let name = resolve_name_cstring(name, "name")?;
    let value = unsafe { value.as_slice()? };
    let rc = unsafe {
        fsetxattr_fd(
            fd,
            name.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            flags.0 as libc::c_int,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("fsetxattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.listxattrBytes`.
pub(crate) unsafe fn destack_fs_listxattr_bytes(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: PathBytes,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = resolve_path_bytes_cstring(path, "path")?;
    let size = unsafe { listxattr_path(path.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("listxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe { listxattr_path(path.as_ptr(), buffer.as_mut_ptr() as *mut _, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("listxattr", None));
    }
    unsafe {
        *out = decode_xattr_list(context, buffer)?;
    }
    Ok(())
}

/// Binding implementation for `destack.fs.listxattrUtf16`.
pub(crate) unsafe fn destack_fs_listxattr_utf16(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: PathUtf16,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = resolve_path_utf16_cstring(path, "path")?;
    let size = unsafe { listxattr_path(path.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("listxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe { listxattr_path(path.as_ptr(), buffer.as_mut_ptr() as *mut _, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("listxattr", None));
    }
    unsafe {
        *out = decode_xattr_list(context, buffer)?;
    }
    Ok(())
}

/// Binding implementation for `destack.fs.llistxattrBytes`.
pub(crate) unsafe fn destack_fs_llistxattr_bytes(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: PathBytes,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = resolve_path_bytes_cstring(path, "path")?;
    let size = unsafe { llistxattr_path(path.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("llistxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe { llistxattr_path(path.as_ptr(), buffer.as_mut_ptr() as *mut _, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("llistxattr", None));
    }
    unsafe {
        *out = decode_xattr_list(context, buffer)?;
    }
    Ok(())
}

/// Binding implementation for `destack.fs.llistxattrUtf16`.
pub(crate) unsafe fn destack_fs_llistxattr_utf16(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: PathUtf16,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = resolve_path_utf16_cstring(path, "path")?;
    let size = unsafe { llistxattr_path(path.as_ptr(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("llistxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe { llistxattr_path(path.as_ptr(), buffer.as_mut_ptr() as *mut _, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("llistxattr", None));
    }
    unsafe {
        *out = decode_xattr_list(context, buffer)?;
    }
    Ok(())
}

/// Binding implementation for `destack.fs.flistxattr`.
pub(crate) unsafe fn destack_fs_flistxattr_handle(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let fd = file_descriptor(context, handle)?;
    let size = unsafe { flistxattr_fd(fd, std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(core_platform::io_error("flistxattr", None));
    }
    let mut buffer = vec![0u8; size as usize];
    let rc = unsafe { flistxattr_fd(fd, buffer.as_mut_ptr() as *mut _, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("flistxattr", None));
    }
    unsafe {
        *out = decode_xattr_list(context, buffer)?;
    }
    Ok(())
}

/// Binding implementation for `destack.fs.removexattrBytes`.
pub(crate) unsafe fn destack_fs_removexattr_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = context;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let rc = unsafe { removexattr_path(path.as_ptr(), name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("removexattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.removexattrUtf16`.
pub(crate) unsafe fn destack_fs_removexattr_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = context;
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let rc = unsafe { removexattr_path(path.as_ptr(), name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("removexattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.lremovexattrBytes`.
pub(crate) unsafe fn destack_fs_lremovexattr_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = context;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let rc = unsafe { lremovexattr_path(path.as_ptr(), name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("lremovexattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.lremovexattrUtf16`.
pub(crate) unsafe fn destack_fs_lremovexattr_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = context;
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_cstring(name, "name")?;
    let rc = unsafe { lremovexattr_path(path.as_ptr(), name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("lremovexattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.fremovexattr`.
pub(crate) unsafe fn destack_fs_fremovexattr_handle(
    context: &RuntimeCallContext,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = context;
    let fd = file_descriptor(context, handle)?;
    let name = resolve_name_cstring(name, "name")?;
    let rc = unsafe { fremovexattr_fd(fd, name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("fremovexattr", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.mmapFile`.
pub(crate) unsafe fn destack_fs_mmap_file(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve file descriptor
    let fd = file_descriptor(context, handle)?;

    // map flags and protections
    let mut native_flags = 0;
    if flags.0 & 0x1 != 0 {
        native_flags |= libc::MAP_SHARED;
    }
    if flags.0 & 0x2 != 0 {
        native_flags |= libc::MAP_PRIVATE;
    }
    if flags.0 & 0x10 != 0 {
        native_flags |= libc::MAP_FIXED;
    }
    if flags.0 & 0x20 != 0 {
        native_flags |= libc::MAP_ANON;
    }
    let mut native_prot = 0;
    if prot.0 & 0x1 != 0 {
        native_prot |= libc::PROT_READ;
    }
    if prot.0 & 0x2 != 0 {
        native_prot |= libc::PROT_WRITE;
    }
    if prot.0 & 0x4 != 0 {
        native_prot |= libc::PROT_EXEC;
    }

    // map the file
    let offset = offset_to_off_t(offset)?;
    let length = length.0 as libc::size_t;
    let mapping = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            length,
            native_prot,
            native_flags,
            fd,
            offset,
        )
    };
    if mapping == libc::MAP_FAILED {
        return Err(core_platform::io_error("mmap", None));
    }

    unsafe {
        *out = NativeSlice {
            data: mapping as *mut u8,
            len: length as u32,
        };
    }

    Ok(())
}

/// Binding implementation for `destack.fs.mmapAnonymous`.
pub(crate) unsafe fn destack_fs_mmap_anonymous(
    _context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // map flags and protections
    let mut native_flags = libc::MAP_ANON;
    if flags.0 & 0x1 != 0 {
        native_flags |= libc::MAP_SHARED;
    }
    if flags.0 & 0x2 != 0 {
        native_flags |= libc::MAP_PRIVATE;
    }
    if flags.0 & 0x10 != 0 {
        native_flags |= libc::MAP_FIXED;
    }
    let mut native_prot = 0;
    if prot.0 & 0x1 != 0 {
        native_prot |= libc::PROT_READ;
    }
    if prot.0 & 0x2 != 0 {
        native_prot |= libc::PROT_WRITE;
    }
    if prot.0 & 0x4 != 0 {
        native_prot |= libc::PROT_EXEC;
    }

    let length = length.0 as libc::size_t;
    let mapping = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            length,
            native_prot,
            native_flags,
            -1,
            0,
        )
    };
    if mapping == libc::MAP_FAILED {
        return Err(core_platform::io_error("mmap", None));
    }

    unsafe {
        *out = NativeSlice {
            data: mapping as *mut u8,
            len: length as u32,
        };
    }

    Ok(())
}

/// Binding implementation for `destack.fs.munmap`.
pub(crate) unsafe fn destack_fs_munmap(
    _context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let slice = unsafe { mapping.as_slice()? };
    if slice.is_empty() {
        return Ok(());
    }
    let rc = unsafe { libc::munmap(mapping.data as *mut libc::c_void, mapping.len as usize) };
    if rc != 0 {
        return Err(core_platform::io_error("munmap", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.mprotect`.
pub(crate) unsafe fn destack_fs_mprotect(
    _context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    prot: MmapProt,
) -> RuntimeResult<()> {
    let _ = mapping;
    let mut native_prot = 0;
    if prot.0 & 0x1 != 0 {
        native_prot |= libc::PROT_READ;
    }
    if prot.0 & 0x2 != 0 {
        native_prot |= libc::PROT_WRITE;
    }
    if prot.0 & 0x4 != 0 {
        native_prot |= libc::PROT_EXEC;
    }
    let rc = unsafe {
        libc::mprotect(
            mapping.data as *mut libc::c_void,
            mapping.len as usize,
            native_prot,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("mprotect", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.msync`.
pub(crate) unsafe fn destack_fs_msync(
    _context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let mut native_flags = 0;
    if flags.0 & 0x1 != 0 {
        native_flags |= libc::MS_SYNC;
    }
    if flags.0 & 0x2 != 0 {
        native_flags |= libc::MS_ASYNC;
    }
    if flags.0 & 0x4 != 0 {
        native_flags |= libc::MS_INVALIDATE;
    }
    let rc = unsafe {
        libc::msync(
            mapping.data as *mut libc::c_void,
            mapping.len as usize,
            native_flags,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("msync", None));
    }
    Ok(())
}

/// Binding implementation for `destack.fs.madvise`.
pub(crate) unsafe fn destack_fs_madvise(
    _context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    advice: MmapAdvice,
) -> RuntimeResult<()> {
    let advice = match advice {
        MmapAdvice::Normal => libc::MADV_NORMAL,
        MmapAdvice::Sequential => libc::MADV_SEQUENTIAL,
        MmapAdvice::Random => libc::MADV_RANDOM,
        MmapAdvice::WillNeed => libc::MADV_WILLNEED,
        MmapAdvice::DontNeed => libc::MADV_DONTNEED,
    };
    let rc = unsafe {
        libc::madvise(
            mapping.data as *mut libc::c_void,
            mapping.len as usize,
            advice,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("madvise", None));
    }
    Ok(())
}
