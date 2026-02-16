#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, net as platform_net, *};
use crate::runtime::RuntimeCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;

/// Copy a file.
///
/// Copy file contents and requested metadata behavior from source path to destination path.
/// Copy flags control overwrite behavior and host fast-copy strategies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses copy_file_range/copy fallback on Unix and CopyFileW/CopyFile2 on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    _context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: CopyFlags,
) -> RuntimeResult<()> {
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

/// Copy a file.
///
/// Copy file contents and requested metadata behavior from source path to destination path.
/// Copy flags control overwrite behavior and host fast-copy strategies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses copy_file_range/copy fallback on Unix and CopyFileW/CopyFile2 on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    _context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported copyfile flags",
        ))
        .boxed());
    }

    // report unsupported copyfile calls on non-windows platforms
    let _ = (from, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfileUtf16")).boxed())
}

/// Copy a range between file descriptors.
///
/// Copy bytes from one file descriptor range into another descriptor range.
/// Source and destination offsets are applied exactly as provided to the host operation.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses copy_file_range(2) on linux and runtime copy fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
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

/// Send file data to a socket.
///
/// Transfer file bytes directly from storage-backed pages to a socket endpoint.
/// Host fast-path behavior may bypass user-space copies when supported.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses sendfile(2) on Unix variants and TransmitFile or copy fallback on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
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
pub(super) unsafe fn getxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::getxattr(path, name, value, size, 0, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn getxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::getxattr(path, name, value, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn lgetxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::getxattr(path, name, value, size, 0, libc::XATTR_NOFOLLOW) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn lgetxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::lgetxattr(path, name, value, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn fgetxattr_fd(
    fd: libc::c_int,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::fgetxattr(fd, name, value, size, 0, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn fgetxattr_fd(
    fd: libc::c_int,
    name: *const libc::c_char,
    value: *mut libc::c_void,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::fgetxattr(fd, name, value, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn listxattr_path(
    path: *const libc::c_char,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::listxattr(path, list, size, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn listxattr_path(
    path: *const libc::c_char,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::listxattr(path, list, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn llistxattr_path(
    path: *const libc::c_char,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::listxattr(path, list, size, libc::XATTR_NOFOLLOW) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn llistxattr_path(
    path: *const libc::c_char,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::llistxattr(path, list, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn flistxattr_fd(
    fd: libc::c_int,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::flistxattr(fd, list, size, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn flistxattr_fd(
    fd: libc::c_int,
    list: *mut libc::c_char,
    size: usize,
) -> libc::ssize_t {
    unsafe { libc::flistxattr(fd, list, size) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn setxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::setxattr(path, name, value, size, 0, flags) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn setxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::setxattr(path, name, value, size, flags) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn lsetxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::setxattr(path, name, value, size, 0, flags | libc::XATTR_NOFOLLOW) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn lsetxattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::lsetxattr(path, name, value, size, flags) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn fsetxattr_fd(
    fd: libc::c_int,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::fsetxattr(fd, name, value, size, 0, flags) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn fsetxattr_fd(
    fd: libc::c_int,
    name: *const libc::c_char,
    value: *const libc::c_void,
    size: usize,
    flags: libc::c_int,
) -> libc::c_int {
    unsafe { libc::fsetxattr(fd, name, value, size, flags) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn removexattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
) -> libc::c_int {
    unsafe { libc::removexattr(path, name, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn removexattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
) -> libc::c_int {
    unsafe { libc::removexattr(path, name) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn lremovexattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
) -> libc::c_int {
    unsafe { libc::removexattr(path, name, libc::XATTR_NOFOLLOW) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn lremovexattr_path(
    path: *const libc::c_char,
    name: *const libc::c_char,
) -> libc::c_int {
    unsafe { libc::lremovexattr(path, name) }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(super) unsafe fn fremovexattr_fd(fd: libc::c_int, name: *const libc::c_char) -> libc::c_int {
    unsafe { libc::fremovexattr(fd, name, 0) }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(super) unsafe fn fremovexattr_fd(fd: libc::c_int, name: *const libc::c_char) -> libc::c_int {
    unsafe { libc::fremovexattr(fd, name) }
}

pub(super) fn decode_xattr_list(
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

/// Copy a file.
///
/// Copy file contents and requested metadata behavior from source path to destination path.
/// Copy flags control overwrite behavior and host fast-copy strategies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses copy_file_range/copy fallback on Unix and CopyFileW/CopyFile2 on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_copyfile(
    context: &RuntimeCallContext,
    from: OsPath,
    to: OsPath,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_copyfile_bytes(context, from, to, flags) },
        |from, to| unsafe { destack_fs_copyfile_utf16(context, from, to, flags) },
    )
}
