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
            .insert(binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Open a file and return a handle.
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
            .insert(binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = DirectoryHandle(resource_id);
    }
    Ok(())
}

/// Open a directory and return a handle.
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
        .insert(binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = FileHandle(handle);
    }
    Ok(())
}

/// Open a file relative to a directory handle.
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
                .insert(binding.world(), entry, Some(binding.engine()));
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
