use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, net as platform_net, *};
use crate::runtime::BindingCallContext;

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;

/// Copyfile flag to reject replacing an existing destination.
const COPYFILE_FAIL_IF_EXISTS: u32 = 0x1;

/// Copy one descriptor range through the portable pread or pwrite fallback.
fn copy_range_fallback(
    src_fd: RawFd,
    mut src_offset: libc::off_t,
    dst_fd: RawFd,
    mut dst_offset: libc::off_t,
    length: u64,
) -> RuntimeResult<u64> {
    // copy the requested range in bounded chunks
    let mut remaining = length;
    let mut total = 0u64;
    let buffer_length = core_fs::copy_fallback_buffer_length(length);
    let mut buffer = vec![0u8; buffer_length];

    while remaining > 0 {
        // read one source chunk
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

        // write the full chunk to the destination
        let bytes = rc as usize;
        let mut written = 0usize;
        while written < bytes {
            let wc = unsafe {
                libc::pwrite(
                    dst_fd,
                    buffer[written..bytes].as_ptr() as *const libc::c_void,
                    bytes - written,
                    dst_offset + written as libc::off_t,
                )
            };
            if wc < 0 {
                return Err(core_platform::io_error("pwrite", None));
            }
            if wc == 0 {
                return Err(RuntimeError::from(PlatformError::io(
                    "copy_file_range fallback write returned zero bytes".to_string(),
                ))
                .boxed());
            }

            written = written.saturating_add(wc as usize);
        }

        src_offset += bytes as libc::off_t;
        dst_offset += bytes as libc::off_t;
        total += bytes as u64;
        remaining = remaining.saturating_sub(bytes as u64);
    }

    Ok(total)
}

/// Return whether one copy_file_range failure should fall back to userspace copying.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn should_fallback_from_copy_file_range(errno: i32) -> bool {
    matches!(
        errno,
        libc::ENOSYS | libc::EXDEV | libc::EINVAL | libc::EOPNOTSUPP | libc::EPERM
    )
}

/// Copy one descriptor range through copy_file_range when the host supports it.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn copy_range_with_kernel_fast_path(
    src_fd: RawFd,
    mut src_offset: libc::off_t,
    dst_fd: RawFd,
    mut dst_offset: libc::off_t,
    length: u64,
) -> RuntimeResult<u64> {
    // copy the requested range and retain explicit offsets
    let mut remaining = length;
    let mut total = 0u64;

    while remaining > 0 {
        // bound one syscall length to host size_t range
        let chunk = remaining.min(usize::MAX as u64) as usize;
        #[cfg(target_os = "linux")]
        let rc = unsafe {
            libc::copy_file_range(
                src_fd,
                &mut src_offset as *mut libc::off_t,
                dst_fd,
                &mut dst_offset as *mut libc::off_t,
                chunk,
                0,
            )
        };
        #[cfg(target_os = "android")]
        let rc = unsafe {
            libc::syscall(
                libc::SYS_copy_file_range,
                src_fd,
                &mut src_offset as *mut libc::off_t,
                dst_fd,
                &mut dst_offset as *mut libc::off_t,
                chunk,
                0,
            ) as libc::ssize_t
        };

        // fall back to the portable loop only when the kernel rejects the primitive itself
        if rc < 0 {
            let errno = core_platform::get_errno();
            if total == 0 && should_fallback_from_copy_file_range(errno) {
                return copy_range_fallback(src_fd, src_offset, dst_fd, dst_offset, remaining);
            }

            return Err(core_platform::io_error("copy_file_range", None));
        }

        // stop at eof
        if rc == 0 {
            break;
        }

        let copied = rc as u64;
        total += copied;
        remaining = remaining.saturating_sub(copied);
    }

    Ok(total)
}

/// Copy one descriptor range on hosts without copy_file_range support.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn copy_range_with_kernel_fast_path(
    src_fd: RawFd,
    src_offset: libc::off_t,
    dst_fd: RawFd,
    dst_offset: libc::off_t,
    length: u64,
) -> RuntimeResult<u64> {
    copy_range_fallback(src_fd, src_offset, dst_fd, dst_offset, length)
}

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    _binding: &BindingCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // resolve the source and destination paths
    let from_path = resolve_path_bytes(from, "from")?;
    let to_path = resolve_path_bytes(to, "to")?;

    // reject unsupported flags
    let supported_flags = COPYFILE_FAIL_IF_EXISTS;
    if flags.0 & !supported_flags != 0 {
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
    let mut dst_open_flags = libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC;
    if flags.0 & COPYFILE_FAIL_IF_EXISTS != 0 {
        dst_open_flags |= libc::O_EXCL;
    }
    let dst_fd = unsafe { libc::open(to_c.as_ptr(), dst_open_flags, mode as libc::c_uint) };
    if dst_fd < 0 {
        return Err(core_platform::io_error(
            "open",
            Some(to_path.to_string_lossy().as_ref()),
        ));
    }
    let _dst_guard = FdGuard(dst_fd);

    // copy the file contents with the best host primitive available
    copy_range_with_kernel_fast_path(src_fd, 0, dst_fd, 0, stat.st_size.max(0) as u64)?;

    // restore the source permission bits after open so umask does not leak into the copy
    let rc = unsafe { libc::fchmod(dst_fd, mode as libc::mode_t) };
    if rc != 0 {
        return Err(core_platform::io_error(
            "fchmod",
            Some(to_path.to_string_lossy().as_ref()),
        ));
    }

    Ok(())
}

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    binding: &BindingCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags
    let supported_flags = COPYFILE_FAIL_IF_EXISTS;
    if flags.0 & !supported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported copyfile flags",
        ))
        .boxed());
    }

    // copy the file by converting utf16 paths to bytes
    core_fs::with_utf16_pair_as_bytes(from, to, "path", |from, to| unsafe {
        destack_fs_copyfile_bytes(binding, from, to, flags)
    })
}

/// Copy a range between file descriptors.
pub(crate) unsafe fn destack_fs_copy_file_range(
    binding: &BindingCallContext,
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
    let src_fd = file_descriptor(binding, src)?;
    let dst_fd = file_descriptor(binding, dst)?;
    let src_offset = offset_to_off_t(src_offset)?;
    let dst_offset = offset_to_off_t(dst_offset)?;

    // copy the requested range with the best host primitive available
    let total = copy_range_with_kernel_fast_path(src_fd, src_offset, dst_fd, dst_offset, length.0)?;

    unsafe {
        *out = total;
    }

    Ok(())
}

/// Send file data to a socket.
pub(crate) unsafe fn destack_fs_sendfile(
    binding: &BindingCallContext,
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
    let fd = file_descriptor(binding, file)?;
    let sock_fd = platform_net::core::require_resource(
        binding,
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
            let buffer_length = core_fs::copy_fallback_buffer_length(length.0);
            let mut buffer = vec![0u8; buffer_length];

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

pub(super) fn decode_xattr_list_bytes(
    binding: &BindingCallContext,
    buffer: Vec<u8>,
) -> RuntimeResult<NativeArray<NativeArray<u8>>> {
    // split on nul separators
    let mut names = Vec::new();
    for entry in buffer.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        names.push(binding.store_array_copy(entry));
    }

    Ok(binding.store_array(names))
}

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile(
    binding: &BindingCallContext,
    from: OsPath,
    to: OsPath,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref_pair(
        from,
        to,
        "path",
        |from, to| unsafe { destack_fs_copyfile_bytes(binding, from, to, flags) },
        |from, to| unsafe { destack_fs_copyfile_utf16(binding, from, to, flags) },
    )
}
