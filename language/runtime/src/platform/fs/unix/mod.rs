use std::path::PathBuf;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    AccessMode, AtFlags, Dirent, DirentKind, FileLockFlags, FileMode, FileOffset, FileSize,
    OpenFlags, PathBytes, PathBytesAbi, PathUtf16, Stat, StatFs, SymlinkType, core as core_fs,
};
use crate::platform::resource::{
    DirectoryHandle, FileHandle, ResourceEntry, ResourceFinalizer, ResourceKind,
};
use crate::platform::{NativeArray, NativeSlice, PlatformError, ResourceId};
use crate::runtime::RuntimeCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
use linux as os;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;

#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "ios")]
use ios as os;

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "freebsd")]
use freebsd as os;

#[cfg(target_os = "openbsd")]
mod openbsd;
#[cfg(target_os = "openbsd")]
use openbsd as os;

#[cfg(target_os = "netbsd")]
mod netbsd;
#[cfg(target_os = "netbsd")]
use netbsd as os;

#[cfg(target_os = "dragonfly")]
mod dragonfly;
#[cfg(target_os = "dragonfly")]
use dragonfly as os;

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
    let bytes = unsafe { path.0.as_slice()? };
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed());
    }

    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

/// Resolve a raw byte path into a CString for libc calls.
fn resolve_path_bytes_cstring(path: PathBytes, name: &str) -> RuntimeResult<CString> {
    let bytes = unsafe { path.0.as_slice()? };
    CString::new(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "path contains nul byte",
        ))
        .boxed()
    })
}

/// Decode raw bytes into a string.
fn string_from_bytes(bytes: &[u8], name: &str) -> RuntimeResult<String> {
    std::str::from_utf8(bytes)
        .map(|value| value.to_string())
        .map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                name,
                "path contains invalid utf8",
            ))
            .boxed()
        })
}

/// Resolve a resource entry for a file handle.
fn file_descriptor(context: &RuntimeCallContext, handle: FileHandle) -> RuntimeResult<RawFd> {
    core_fs::require_resource(context, handle.0, ResourceKind::File, "file", |entry| {
        entry.fd()
    })
}

/// Resolve a directory resource from a handle.
fn directory_resource(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<DirectoryResource> {
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
        },
    )
}

/// Map a libc dirent type to our dirent kind.
fn dirent_kind_from_type(kind: u8) -> DirentKind {
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
    if offset.0 > i64::MAX as u64 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "offset",
            "offset out of range",
        ))
        .boxed());
    }

    Ok(offset.0 as libc::off_t)
}

/// Build a runtime error from the last OS error.
fn last_os_error(syscall: &str, path: Option<&str>) -> Box<RuntimeError> {
    let error = std::io::Error::last_os_error();
    let errno = error.raw_os_error();
    let message = format!("{syscall} failed: {error}");
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        errno,
        Some(syscall.to_string()),
        path.map(|path| path.to_string()),
        message,
    ))
    .boxed()
}

/// Convert libc stat into the ABI Stat shape.
fn stat_from_libc(stat: libc::stat) -> Stat {
    let (atime_ns, mtime_ns, ctime_ns, birthtime_ns) = os::stat_times(stat);
    Stat {
        dev: stat_u64(stat.st_dev),
        ino: stat_u64(stat.st_ino),
        mode: stat.st_mode as u32,
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

/// Return the current errno pointer on unix.
fn errno_location() -> *mut libc::c_int {
    os::errno_location()
}

/// Set errno to a specific value.
fn set_errno(value: libc::c_int) {
    unsafe {
        *errno_location() = value;
    }
}

/// Read the current errno value.
fn get_errno() -> libc::c_int {
    unsafe { *errno_location() }
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
            return Err(last_os_error("readlinkat", None));
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
    libc::timespec {
        tv_sec: (nanos / 1_000_000_000) as libc::time_t,
        tv_nsec: (nanos % 1_000_000_000) as libc::c_long,
    }
}

/// Stub for destack.fs.access.
/// Stub for destack.fs.accessBytes.
pub(crate) unsafe fn destack_fs_access_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = context;

    // run the access check on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::access(c_path.as_ptr(), mode.0 as libc::c_int) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("access", None))
    }

    // report unsupported access checks on non-unix platforms
}

/// Stub for destack.fs.accessUtf16.
pub(crate) unsafe fn destack_fs_access_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported access checks on non-windows platforms
    {
        let _ = (path, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessUtf16")).boxed())
    }

    // run the access check on windows platforms
}

/// Stub for destack.fs.chmod.
/// Stub for destack.fs.chmodBytes.
pub(crate) unsafe fn destack_fs_chmod_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = context;

    // apply permissions on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::chmod(c_path.as_ptr(), mode.0 as libc::mode_t) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("chmod", None))
    }

    // report unsupported chmod calls on non-unix platforms
}

/// Stub for destack.fs.chmodUtf16.
pub(crate) unsafe fn destack_fs_chmod_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported chmod calls on non-windows platforms
    {
        let _ = (path, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmodUtf16")).boxed())
    }

    // apply permissions on windows platforms
}

/// Stub for destack.fs.chown.
/// Stub for destack.fs.chownBytes.
pub(crate) unsafe fn destack_fs_chown_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = context;

    // apply ownership on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::chown(c_path.as_ptr(), uid, gid) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("chown", None))
    }

    // report unsupported chown calls on non-unix platforms
}

/// Stub for destack.fs.chownUtf16.
pub(crate) unsafe fn destack_fs_chown_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported chown calls on non-windows platforms
    {
        let _ = (path, uid, gid);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownUtf16")).boxed())
    }

    // apply ownership on windows platforms
}

/// Stub for destack.fs.close.
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

/// Stub for destack.fs.closedir.
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

/// Stub for destack.fs.copyfile.
/// Stub for destack.fs.copyfileBytes.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = context;

    // resolve the source and destination paths
    let from_path = resolve_path_bytes(from, "from")?;
    let to_path = resolve_path_bytes(to, "to")?;

    // reject unsupported flags
    if flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported copyfile flags",
        ))
        .boxed());
    }

    // perform the file copy on unix platforms
    {
        std::fs::copy(&from_path, &to_path).map_err(|error| {
            RuntimeError::from(PlatformError::io_with(
                None,
                None,
                error.raw_os_error(),
                Some("copy".to_string()),
                None,
                format!("copyfile failed: {error}"),
            ))
            .boxed()
        })?;
        Ok(())
    }
}

/// Stub for destack.fs.copyfileUtf16.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = context;

    // reject unsupported flags
    if flags != 0 {
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

/// Stub for destack.fs.fchmod.
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

        Err(last_os_error("fchmod", None))
    }
}

/// Stub for destack.fs.fchown.
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

        Err(last_os_error("fchown", None))
    }
}

/// Stub for destack.fs.fdatasync.
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

        Err(last_os_error("fdatasync", None))
    }
}

/// Stub for destack.fs.fstat.
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
            return Err(last_os_error("fstat", None));
        }

        unsafe {
            *out = stat_from_libc(stat);
        }

        Ok(())
    }
}

/// Stub for destack.fs.fstatfs.
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

/// Stub for destack.fs.fsync.
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

        Err(last_os_error("fsync", None))
    }
}

/// Stub for destack.fs.ftruncate.
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

        Err(last_os_error("ftruncate", None))
    }
}

/// Stub for destack.fs.futimes.
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

        Err(last_os_error("futimens", None))
    }
}

/// Stub for destack.fs.link.
/// Stub for destack.fs.linkBytes.
pub(crate) unsafe fn destack_fs_link_bytes(
    context: &RuntimeCallContext,
    existingpath: PathBytes,
    newpath: PathBytes,
) -> RuntimeResult<()> {
    let _ = context;

    // create the link on unix platforms
    {
        let existing = resolve_path_bytes_cstring(existingpath, "existingPath")?;
        let newpath = resolve_path_bytes_cstring(newpath, "newPath")?;
        let rc = unsafe { libc::link(existing.as_ptr(), newpath.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("link", None))
    }
}

/// Stub for destack.fs.linkUtf16.
pub(crate) unsafe fn destack_fs_link_utf16(
    context: &RuntimeCallContext,
    existingpath: PathUtf16,
    newpath: PathUtf16,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported link calls on non-windows platforms
    {
        let _ = (existingpath, newpath);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkUtf16")).boxed())
    }

    // create the link on windows platforms
}

/// Stub for destack.fs.lstat.
/// Stub for destack.fs.lstatBytes.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
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
            return Err(last_os_error("lstat", None));
        }

        unsafe {
            *out = stat_from_libc(stat);
        }

        Ok(())
    }
}

/// Stub for destack.fs.lstatUtf16.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
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

/// Stub for destack.fs.lutimes.
/// Stub for destack.fs.lutimesBytes.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
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

        Err(last_os_error("utimensat", None))
    }
}

/// Stub for destack.fs.lutimesUtf16.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported lutimes calls on non-windows platforms
    {
        let _ = (path, atimens, mtimens);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesUtf16")).boxed())
    }

    // apply timestamps on windows platforms
}

/// Stub for destack.fs.mkdir.
/// Stub for destack.fs.mkdtemp.
/// Stub for destack.fs.mkdtempBytes.
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
            return Err(last_os_error("mkdtemp", None));
        }

        let value = unsafe { CStr::from_ptr(result) }.to_bytes().to_vec();
        unsafe {
            *out = PathBytesAbi::<NativeAbi>(context.store_array(value));
        }

        Ok(())
    }
}

/// Stub for destack.fs.mkdtempUtf16.
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

/// Stub for destack.fs.open.
/// Stub for destack.fs.openBytes.
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
            return Err(last_os_error("open", None));
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

/// Stub for destack.fs.openUtf16.
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

/// Stub for destack.fs.opendir.
/// Stub for destack.fs.opendirBytes.
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
            return Err(last_os_error("opendir", None));
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

/// Stub for destack.fs.opendirUtf16.
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

/// Stub for destack.fs.read.
pub(crate) unsafe fn destack_fs_read(
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
    {
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
            return Err(last_os_error("pread", None));
        }

        unsafe {
            *out = rc as u64;
        }

        Ok(())
    }
}

/// Stub for destack.fs.readdir.
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
            return Err(last_os_error("dup", None));
        }

        let dirp = unsafe { libc::fdopendir(dup_fd) };
        if dirp.is_null() {
            unsafe {
                libc::close(dup_fd);
            }
            return Err(last_os_error("fdopendir", None));
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
            set_errno(0);
            let entry = unsafe { libc::readdir(dirp) };
            if entry.is_null() {
                if get_errno() != 0 {
                    return Err(last_os_error(
                        "readdir",
                        Some(resource.path.to_string_lossy().as_ref()),
                    ));
                }
                break;
            }

            let name_ptr = unsafe { (*entry).d_name.as_ptr() };
            let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
            let name_bytes = name.to_bytes();
            let name_string = string_from_bytes(name_bytes, "name")?;
            if name_string == "." || name_string == ".." {
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
            dirents.push(Dirent {
                name: context.store_string(&name_string),
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

/// Stub for destack.fs.readlink.
/// Stub for destack.fs.readlinkBytes.
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

/// Stub for destack.fs.readlinkUtf16.
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

/// Stub for destack.fs.readv.
pub(crate) unsafe fn destack_fs_readv(
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
    {
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
            return Err(last_os_error("preadv", None));
        }

        unsafe {
            *out = rc as u64;
        }

        Ok(())
    }
}

/// Stub for destack.fs.realpath.
/// Stub for destack.fs.realpathBytes.
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
            return Err(last_os_error("realpath", None));
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

/// Stub for destack.fs.realpathUtf16.
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

/// Stub for destack.fs.rename.
/// Stub for destack.fs.renameBytes.
pub(crate) unsafe fn destack_fs_rename_bytes(
    context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    let _ = context;

    // rename the path on unix platforms
    {
        let from = resolve_path_bytes_cstring(from, "from")?;
        let to = resolve_path_bytes_cstring(to, "to")?;
        let result = unsafe { libc::rename(from.as_ptr(), to.as_ptr()) };
        if result != 0 {
            return Err(last_os_error("rename", None));
        }

        Ok(())
    }
}

/// Stub for destack.fs.renameUtf16.
pub(crate) unsafe fn destack_fs_rename_utf16(
    context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported rename calls on non-windows platforms
    {
        let _ = (from, to);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameUtf16")).boxed())
    }

    // rename the path on windows platforms
}

/// Stub for destack.fs.rmdir.
/// Stub for destack.fs.rmdirBytes.
pub(crate) unsafe fn destack_fs_rmdir_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = context;

    // remove the directory on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::rmdir(c_path.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("rmdir", None))
    }
}

/// Stub for destack.fs.rmdirUtf16.
pub(crate) unsafe fn destack_fs_rmdir_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported rmdir calls on non-windows platforms
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirUtf16")).boxed())
    }

    // remove the directory on windows platforms
}

/// Stub for destack.fs.stat.
/// Stub for destack.fs.statBytes.
pub(crate) unsafe fn destack_fs_stat_bytes(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
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
            return Err(last_os_error("stat", None));
        }

        unsafe {
            *out = stat_from_libc(stat);
        }

        Ok(())
    }
}

/// Stub for destack.fs.statUtf16.
pub(crate) unsafe fn destack_fs_stat_utf16(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
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

/// Stub for destack.fs.statfs.
/// Stub for destack.fs.statfsBytes.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
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

/// Stub for destack.fs.statfsUtf16.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    path: PathUtf16,
) -> RuntimeResult<()> {
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

/// Stub for destack.fs.symlink.
/// Stub for destack.fs.symlinkBytes.
pub(crate) unsafe fn destack_fs_symlink_bytes(
    context: &RuntimeCallContext,
    target: PathBytes,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = kind;

    // create the symlink on unix platforms
    {
        let target = resolve_path_bytes_cstring(target, "target")?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::symlink(target.as_ptr(), path.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("symlink", None))
    }
}

/// Stub for destack.fs.symlinkUtf16.
pub(crate) unsafe fn destack_fs_symlink_utf16(
    context: &RuntimeCallContext,
    target: PathUtf16,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = kind;

    // report unsupported symlink calls on non-windows platforms
    {
        let _ = (target, path);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkUtf16")).boxed())
    }

    // create the symlink on windows platforms
}

/// Stub for destack.fs.truncate.
/// Stub for destack.fs.truncateBytes.
pub(crate) unsafe fn destack_fs_truncate_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = context;

    // truncate the file on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let offset = offset_to_off_t(size)?;
        let rc = unsafe { libc::truncate(c_path.as_ptr(), offset) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("truncate", None))
    }
}

/// Stub for destack.fs.truncateUtf16.
pub(crate) unsafe fn destack_fs_truncate_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported truncate calls on non-windows platforms
    {
        let _ = (path, size);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateUtf16")).boxed())
    }

    // truncate the file on windows platforms
}

/// Stub for destack.fs.unlink.
/// Stub for destack.fs.unlinkBytes.
pub(crate) unsafe fn destack_fs_unlink_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = context;

    // unlink the file on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::unlink(c_path.as_ptr()) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("unlink", None))
    }
}

/// Stub for destack.fs.unlinkUtf16.
pub(crate) unsafe fn destack_fs_unlink_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported unlink calls on non-windows platforms
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkUtf16")).boxed())
    }

    // unlink the file on windows platforms
}

/// Stub for destack.fs.utimes.
/// Stub for destack.fs.utimesBytes.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = context;

    // apply timestamps on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
        let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c_path.as_ptr(), times.as_ptr(), 0) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("utimensat", None))
    }
}

/// Stub for destack.fs.utimesUtf16.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported utimes calls on non-windows platforms
    {
        let _ = (path, atimens, mtimens);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesUtf16")).boxed())
    }

    // apply timestamps on windows platforms
}

/// Stub for destack.fs.write.
pub(crate) unsafe fn destack_fs_write(
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
    {
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
            return Err(last_os_error("pwrite", None));
        }

        unsafe {
            *out = rc as u64;
        }

        Ok(())
    }
}

/// Stub for destack.fs.writev.
pub(crate) unsafe fn destack_fs_writev(
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
    {
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
            return Err(last_os_error("pwritev", None));
        }

        unsafe {
            *out = rc as u64;
        }

        Ok(())
    }
}

/// Stub for destack.fs.openat.
/// Stub for destack.fs.openatBytes.
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
            return Err(last_os_error("openat", None));
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

/// Stub for destack.fs.openatUtf16.
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

/// Stub for destack.fs.mkdirBytes.
pub(crate) unsafe fn destack_fs_mkdir_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = context;

    // create the directory on unix platforms
    {
        let c_path = resolve_path_bytes_cstring(path, "path")?;
        let rc = unsafe { libc::mkdir(c_path.as_ptr(), mode.0 as libc::mode_t) };
        if rc == 0 {
            return Ok(());
        }

        Err(last_os_error("mkdir", None))
    }
}

/// Stub for destack.fs.mkdirUtf16.
pub(crate) unsafe fn destack_fs_mkdir_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = context;

    // report unsupported mkdir calls on non-windows platforms
    {
        let _ = (path, mode);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdirUtf16")).boxed())
    }

    // create the directory on windows platforms
}

/// Stub for destack.fs.mkdirat.
/// Stub for destack.fs.mkdiratBytes.
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
            return Err(last_os_error("mkdirat", None));
        }
        Ok(())
    }
}

/// Stub for destack.fs.mkdiratUtf16.
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

/// Stub for destack.fs.renameat.
/// Stub for destack.fs.renameatBytes.
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
            return Err(last_os_error("renameat", None));
        }
        Ok(())
    }
}

/// Stub for destack.fs.renameatUtf16.
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

/// Stub for destack.fs.unlinkat.
/// Stub for destack.fs.unlinkatBytes.
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
            return Err(last_os_error("unlinkat", None));
        }
        Ok(())
    }
}

/// Stub for destack.fs.unlinkatUtf16.
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

/// Stub for destack.fs.linkat.
/// Stub for destack.fs.linkatBytes.
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
            return Err(last_os_error("linkat", None));
        }
        Ok(())
    }
}

/// Stub for destack.fs.linkatUtf16.
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

/// Stub for destack.fs.symlinkat.
/// Stub for destack.fs.symlinkatBytes.
pub(crate) unsafe fn destack_fs_symlinkat_bytes(
    context: &RuntimeCallContext,
    target: PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = kind;

    // create the symlink on unix platforms
    {
        let resource = directory_resource(context, dir)?;
        let target = resolve_path_bytes_cstring(target, "target")?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let result = unsafe { libc::symlinkat(target.as_ptr(), resource.fd, path.as_ptr()) };
        if result != 0 {
            return Err(last_os_error("symlinkat", None));
        }
        Ok(())
    }
}

/// Stub for destack.fs.symlinkatUtf16.
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

/// Stub for destack.fs.readlinkat.
/// Stub for destack.fs.readlinkatBytes.
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

/// Stub for destack.fs.readlinkatUtf16.
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

/// Stub for destack.fs.statat.
/// Stub for destack.fs.statatBytes.
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
            return Err(last_os_error("statat", None));
        }
        let stat = unsafe { stat.assume_init() };
        unsafe {
            *out = stat_from_libc(stat);
        }
        Ok(())
    }
}

/// Stub for destack.fs.statatUtf16.
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

/// Stub for destack.fs.lock.
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
            return Err(last_os_error("flock", None));
        }
        Ok(())
    }
}
