use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::sync::{Arc, Mutex};

/// Linux `openat2` argument payload.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[repr(C)]
struct OpenHow {
    /// Open flags.
    flags: u64,
    /// Create mode.
    mode: u64,
    /// Resolve semantics.
    resolve: u64,
}

/// Open a file and return a handle.
///
/// Open one filesystem entry by path and return a host-backed file handle.
/// Flag interpretation, creation behavior, and inheritance defaults follow host open semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses open(2) on Unix and CreateFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_open_bytes(
    binding: &BindingCallContext,
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
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let open_flags = flags.0 as libc::c_int | libc::O_CLOEXEC;
    let fd = unsafe { libc::open(c_path.as_ptr(), open_flags, mode.0 as libc::c_uint) };
    if fd < 0 {
        return Err(core_platform::io_error("open", None));
    }

    let entry = ResourceEntry::new(ResourceKind::File)
        .with_fd(fd)
        .with_finalizer(FdFinalizer {
            fd,
            directory_stream: None,
        });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Open a file and return a handle.
///
/// Open one filesystem entry by path and return a host-backed file handle.
/// Flag interpretation, creation behavior, and inheritance defaults follow host open semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses open(2) on Unix and CreateFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_open_utf16(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the file by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_open_bytes(binding, out, path, flags, mode)
    })
}

/// Open a directory and return a handle.
///
/// Open the target resource with the requested flags and return the host handle exposed by the kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses opendir/readdir on Unix and FindFirstFileW directory enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_opendir_bytes(
    binding: &BindingCallContext,
    out: *mut DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the directory on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC;
    let fd = unsafe { libc::open(c_path.as_ptr(), flags) };
    if fd < 0 {
        return Err(core_platform::io_error("opendir", None));
    }

    // open one shared directory stream for incremental iteration
    let directory_fd = unsafe { libc::dup(fd) };
    if directory_fd < 0 {
        unsafe {
            libc::close(fd);
        }
        return Err(core_platform::io_error("dup", None));
    }

    let directory_stream = unsafe { libc::fdopendir(directory_fd) };
    if directory_stream.is_null() {
        unsafe {
            libc::close(directory_fd);
            libc::close(fd);
        }
        return Err(core_platform::io_error("fdopendir", None));
    }

    let path_buf = resolve_path_bytes(path, "path")?;
    let resource = DirectoryResource {
        path: path_buf,
        fd,
        iterator: Arc::new(Mutex::new(DirectoryIterator::new(directory_stream))),
    };
    let entry = ResourceEntry::new(ResourceKind::Directory)
        .with_payload(resource)
        .with_finalizer(FdFinalizer {
            fd,
            directory_stream: Some(directory_stream as usize),
        });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = DirectoryHandle(resource_id);
    }
    Ok(())
}

/// Open a directory and return a handle.
///
/// Open the target resource with the requested flags and return the host handle exposed by the kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses opendir/readdir on Unix and FindFirstFileW directory enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_opendir_utf16(
    binding: &BindingCallContext,
    out: *mut DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the directory by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_opendir_bytes(binding, out, path)
    })
}

/// Open a file relative to a directory handle.
///
/// Open one filesystem entry resolved relative to an explicit directory handle.
/// This avoids ambient current-working-directory resolution and keeps caller-controlled base directory scope.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat(2) on Unix and NtCreateFile relative opens on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat_bytes(
    binding: &BindingCallContext,
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
    let resource = directory_resource(binding, dir)?;
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
        .with_finalizer(FdFinalizer {
            fd,
            directory_stream: None,
        });
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = FileHandle(handle);
    }
    Ok(())
}

/// Open a file relative to a directory handle.
///
/// Open one filesystem entry resolved relative to an explicit directory handle.
/// This avoids ambient current-working-directory resolution and keeps caller-controlled base directory scope.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat(2) on Unix and NtCreateFile relative opens on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat_utf16(
    binding: &BindingCallContext,
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

    // open the file by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_openat_bytes(binding, out, dir, path, flags, mode)
    })
}

/// Open a file relative to a directory handle with openat2 semantics.
///
/// Open one filesystem entry relative to an explicit directory handle with resolve policy flags.
/// Resolve behavior is passed through to supported hosts and rejected when the host backend cannot honor requested guarantees.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat2(2) on Linux and Android, falls back to openat semantics on other Unix targets when resolve flags are empty, and maps to openat semantics on Windows with resolve flags rejected.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat2_bytes(
    binding: &BindingCallContext,
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
        let resource = directory_resource(binding, dir)?;
        let path = resolve_path_bytes_cstring(path, "path")?;
        let mut open_how: OpenHow = unsafe { std::mem::zeroed() };
        open_how.flags = how.flags.0 as u64;
        open_how.mode = how.mode.0 as u64;
        open_how.resolve = how.resolve.0;
        let fd = unsafe {
            libc::syscall(
                libc::SYS_openat2,
                resource.fd,
                path.as_ptr(),
                &open_how as *const OpenHow,
                std::mem::size_of::<OpenHow>(),
            )
        } as libc::c_int;
        if fd < 0 {
            return Err(core_platform::io_error("openat2", None));
        }
        let entry = ResourceEntry::new(ResourceKind::File)
            .with_fd(fd)
            .with_finalizer(FdFinalizer {
                fd,
                directory_stream: None,
            });
        let handle =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));
        unsafe {
            *out = FileHandle(handle);
        }
        Ok(())
    }

    // fall back to openat when resolve flags are empty on other unix platforms
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        if how.resolve.0 != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.fs.file.openat2",
            ))
            .boxed());
        }
        unsafe { destack_fs_openat_bytes(binding, out, dir, path, how.flags, how.mode) }
    }
}

/// Open a file relative to a directory handle with openat2 semantics.
///
/// Open one filesystem entry relative to an explicit directory handle with resolve policy flags.
/// Resolve behavior is passed through to supported hosts and rejected when the host backend cannot honor requested guarantees.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat2(2) on Linux and Android, falls back to openat semantics on other Unix targets when resolve flags are empty, and maps to openat semantics on Windows with resolve flags rejected.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat2_utf16(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    how: OpenOptions,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // open the file by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_openat2_bytes(binding, out, dir, path, how)
    })
}

/// Open a directory and return a handle.
///
/// Open the target resource with the requested flags and return the host handle exposed by the kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses opendir/readdir on Unix and FindFirstFileW directory enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_opendir(
    binding: &BindingCallContext,
    out: *mut DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_opendir_bytes(binding, out, path) },
        |path| unsafe { destack_fs_opendir_utf16(binding, out, path) },
    )
}

/// Open a file and return a handle.
///
/// Open one filesystem entry by path and return a host-backed file handle.
/// Flag interpretation, creation behavior, and inheritance defaults follow host open semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses open(2) on Unix and CreateFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_open(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_open_bytes(binding, out, path, flags, mode) },
        |path| unsafe { destack_fs_open_utf16(binding, out, path, flags, mode) },
    )
}

/// Open a file relative to a directory handle.
///
/// Open one filesystem entry resolved relative to an explicit directory handle.
/// This avoids ambient current-working-directory resolution and keeps caller-controlled base directory scope.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat(2) on Unix and NtCreateFile relative opens on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_openat_bytes(binding, out, dir, path, flags, mode) },
        |path| unsafe { destack_fs_openat_utf16(binding, out, dir, path, flags, mode) },
    )
}

/// Open a file relative to a directory handle with openat2 semantics.
///
/// Open one filesystem entry relative to an explicit directory handle with resolve policy flags.
/// Resolve behavior is passed through to supported hosts and rejected when the host backend cannot honor requested guarantees.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat2(2) on Linux and Android, falls back to openat semantics on other Unix targets when resolve flags are empty, and maps to openat semantics on Windows with resolve flags rejected.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat2(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: OsPath,
    how: OpenOptions,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_openat2_bytes(binding, out, dir, path, how) },
        |path| unsafe { destack_fs_openat2_utf16(binding, out, dir, path, how) },
    )
}
