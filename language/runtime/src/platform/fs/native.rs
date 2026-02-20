use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{OsPath, XattrFlags};
use crate::platform::resource::FileHandle;
use crate::platform::{NativeArray, NativeSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

#[allow(unused_imports)]
pub(crate) use super::host::*;

/// Read an extended attribute by path with a raw name payload.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses getxattr(2) on Unix and extended-attribute APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_getxattr_bytes(
    _context: &RuntimeCallContext,
    _out: *mut NativeArray<u8>,
    _path: OsPath,
    _name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.getxattrBytes",
    ))
    .boxed())
}

/// Read an extended attribute without following symlinks, using a raw name payload.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lgetxattr(2) on Unix and reparse-aware xattr query where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lgetxattr_bytes(
    _context: &RuntimeCallContext,
    _out: *mut NativeArray<u8>,
    _path: OsPath,
    _name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lgetxattrBytes",
    ))
    .boxed())
}

/// Set an extended attribute by path with a raw name payload.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses setxattr(2) on Unix and extended-attribute APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_setxattr_bytes(
    _context: &RuntimeCallContext,
    _path: OsPath,
    _name: NativeSlice<u8>,
    _value: NativeSlice<u8>,
    _flags: XattrFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.setxattrBytes",
    ))
    .boxed())
}

/// Set an extended attribute without following symlinks, using a raw name payload.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lsetxattr(2) on Unix and reparse-aware xattr write where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lsetxattr_bytes(
    _context: &RuntimeCallContext,
    _path: OsPath,
    _name: NativeSlice<u8>,
    _value: NativeSlice<u8>,
    _flags: XattrFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lsetxattrBytes",
    ))
    .boxed())
}

/// List extended attribute names by path as raw byte payloads.
///
/// List extended attribute names by path via host kernel APIs.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses listxattr(2) on Unix and xattr enumeration APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_listxattr_bytes(
    _context: &RuntimeCallContext,
    _out: *mut NativeArray<NativeArray<u8>>,
    _path: OsPath,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.listxattrBytes",
    ))
    .boxed())
}

/// List extended attribute names without following symlinks as raw byte payloads.
///
/// List extended attribute names without following symlinks via host kernel APIs.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses llistxattr(2) on Unix and reparse-aware xattr enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_llistxattr_bytes(
    _context: &RuntimeCallContext,
    _out: *mut NativeArray<NativeArray<u8>>,
    _path: OsPath,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.llistxattrBytes",
    ))
    .boxed())
}

/// Remove an extended attribute by path with a raw name payload.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses removexattr(2) on Unix and xattr delete APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_removexattr_bytes(
    _context: &RuntimeCallContext,
    _path: OsPath,
    _name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.removexattrBytes",
    ))
    .boxed())
}

/// Remove an extended attribute without following symlinks, using a raw name payload.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lremovexattr(2) on Unix and reparse-aware xattr delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lremovexattr_bytes(
    _context: &RuntimeCallContext,
    _path: OsPath,
    _name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lremovexattrBytes",
    ))
    .boxed())
}

/// Read an extended attribute by handle with a raw name payload.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fgetxattr(2) on Unix and handle-based xattr query where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fgetxattr_bytes(
    _context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fgetxattrBytes",
    ))
    .boxed())
}

/// Set an extended attribute by handle with a raw name payload.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fsetxattr(2) on Unix and handle-based xattr write where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fsetxattr_bytes(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (handle, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fsetxattrBytes",
    ))
    .boxed())
}

/// List extended attribute names by handle as raw byte payloads.
///
/// List extended attribute names by handle via host kernel APIs.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses flistxattr(2) on Unix and handle-based xattr enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_flistxattr_bytes(
    _context: &RuntimeCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.flistxattrBytes",
    ))
    .boxed())
}

/// Remove an extended attribute by handle with a raw name payload.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fremovexattr(2) on Unix and handle-based xattr delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fremovexattr_bytes(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fremovexattrBytes",
    ))
    .boxed())
}
