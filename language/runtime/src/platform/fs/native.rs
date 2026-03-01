use crate::diagnostic::RuntimeResult;
use crate::platform::fs::{OsPath, XattrFlags};
use crate::platform::resource::FileHandle;
use crate::platform::{NativeArray, NativeSlice};
use crate::runtime::BindingCallContext;

use super::host as host_fs;
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
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_getxattr_bytes(context, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_getxattr_utf16(context, out, path_utf16.utf16, name)
        },
    }
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
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lgetxattr_bytes(context, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lgetxattr_utf16(context, out, path_utf16.utf16, name)
        },
    }
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
    context: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_setxattr_bytes(context, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_setxattr_utf16(context, path_utf16.utf16, name, value, flags)
        },
    }
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
    context: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lsetxattr_bytes(context, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lsetxattr_utf16(context, path_utf16.utf16, name, value, flags)
        },
    }
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
    context: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: OsPath,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_listxattr_bytes(context, out, path_bytes.bytes)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_listxattr_utf16(context, out, path_utf16.utf16)
        },
    }
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
    context: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: OsPath,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_llistxattr_bytes(context, out, path_bytes.bytes)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_llistxattr_utf16(context, out, path_utf16.utf16)
        },
    }
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
    context: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_removexattr_bytes(context, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_removexattr_utf16(context, path_utf16.utf16, name)
        },
    }
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
    context: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lremovexattr_bytes(context, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lremovexattr_utf16(context, path_utf16.utf16, name)
        },
    }
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
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fgetxattr_handle(context, out, handle, name) }
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
    context: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fsetxattr_handle(context, handle, name, value, flags) }
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
    context: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_flistxattr_handle(context, out, handle) }
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
    context: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fremovexattr_handle(context, handle, name) }
}
