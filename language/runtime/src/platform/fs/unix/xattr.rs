#![allow(unused_imports)]

use super::copy::{
    decode_xattr_list_bytes, fgetxattr_fd, flistxattr_fd, fremovexattr_fd, fsetxattr_fd,
    getxattr_path, lgetxattr_path, listxattr_path, llistxattr_path, lremovexattr_path,
    lsetxattr_path, removexattr_path, setxattr_path,
};
use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, net as platform_net, *};
use crate::runtime::BindingCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;

fn xattr_name_slice_from_string(
    context: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<NativeSlice<u8>> {
    // decode the string name
    let name = unsafe { name.as_str()? };

    // store a stable raw name slice
    Ok(context.store_slice(name.as_bytes().to_vec()))
}

fn xattr_name_strings_from_bytes(
    context: &BindingCallContext,
    names: NativeArray<NativeArray<u8>>,
) -> RuntimeResult<NativeArray<NativeStringRef>> {
    // decode the raw name arrays
    let names = unsafe { names.as_slice()? };
    let mut decoded = Vec::with_capacity(names.len());

    // decode each name as utf8
    for name in names {
        let bytes = unsafe { name.as_slice()? };
        let name = std::str::from_utf8(bytes).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "xattr",
                "attribute name is not valid utf8",
            ))
            .boxed()
        })?;
        decoded.push(context.store_string(name));
    }

    Ok(context.store_array(decoded))
}

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
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve path and name
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;

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

/// Read an extended attribute by path.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
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
pub(crate) unsafe fn destack_fs_getxattr_utf16(
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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

/// Read an extended attribute without following symlinks.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
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
pub(crate) unsafe fn destack_fs_lgetxattr_utf16(
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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

/// Read an extended attribute by handle.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
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
pub(crate) unsafe fn destack_fs_fgetxattr_handle(
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let fd = file_descriptor(context, handle)?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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
    _context: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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

/// Set an extended attribute by path.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_setxattr_utf16(
    _context: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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
    _context: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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

/// Set an extended attribute without following symlinks.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_lsetxattr_utf16(
    _context: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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

/// Set an extended attribute by handle.
///
/// Set the requested control value on the descriptor through the native option interface.
/// The binding performs one control transaction and returns the exact host outcome without policy retries.
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
pub(crate) unsafe fn destack_fs_fsetxattr_handle(
    context: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let fd = file_descriptor(context, handle)?;
    let name = resolve_name_bytes_cstring(name, "name")?;
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
        *out = decode_xattr_list_bytes(context, buffer)?;
    }
    Ok(())
}

/// List extended attribute names by path.
///
/// List extended attribute names by path via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_listxattr_utf16(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
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
        *out = decode_xattr_list_bytes(context, buffer)?;
    }
    Ok(())
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
        *out = decode_xattr_list_bytes(context, buffer)?;
    }
    Ok(())
}

/// List extended attribute names without following symlinks.
///
/// List extended attribute names without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_llistxattr_utf16(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
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
        *out = decode_xattr_list_bytes(context, buffer)?;
    }
    Ok(())
}

/// List extended attribute names by handle.
///
/// List extended attribute names by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
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
pub(crate) unsafe fn destack_fs_flistxattr_handle(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
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
        *out = decode_xattr_list_bytes(context, buffer)?;
    }
    Ok(())
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
    _context: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
    let rc = unsafe { removexattr_path(path.as_ptr(), name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("removexattr", None));
    }
    Ok(())
}

/// Remove an extended attribute by path.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_removexattr_utf16(
    _context: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
    let rc = unsafe { removexattr_path(path.as_ptr(), name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("removexattr", None));
    }
    Ok(())
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
    _context: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let path = resolve_path_bytes_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
    let rc = unsafe { lremovexattr_path(path.as_ptr(), name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("lremovexattr", None));
    }
    Ok(())
}

/// Remove an extended attribute without following symlinks.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_lremovexattr_utf16(
    _context: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let path = resolve_path_utf16_cstring(path, "path")?;
    let name = resolve_name_bytes_cstring(name, "name")?;
    let rc = unsafe { lremovexattr_path(path.as_ptr(), name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("lremovexattr", None));
    }
    Ok(())
}

/// Remove an extended attribute by handle.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
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
pub(crate) unsafe fn destack_fs_fremovexattr_handle(
    context: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let fd = file_descriptor(context, handle)?;
    let name = resolve_name_bytes_cstring(name, "name")?;
    let rc = unsafe { fremovexattr_fd(fd, name.as_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error("fremovexattr", None));
    }
    Ok(())
}

/// Read an extended attribute by path.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
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
pub(crate) unsafe fn destack_fs_getxattr(
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch by path encoding
    match path.encoding {
        PathEncoding::Bytes => unsafe { destack_fs_getxattr_bytes(context, out, path.bytes, name) },
        PathEncoding::Utf16 => unsafe { destack_fs_getxattr_utf16(context, out, path.utf16, name) },
    }
}

/// Read an extended attribute without following symlinks.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
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
pub(crate) unsafe fn destack_fs_lgetxattr(
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch by path encoding
    match path.encoding {
        PathEncoding::Bytes => unsafe {
            destack_fs_lgetxattr_bytes(context, out, path.bytes, name)
        },
        PathEncoding::Utf16 => unsafe {
            destack_fs_lgetxattr_utf16(context, out, path.utf16, name)
        },
    }
}

/// Read an extended attribute by handle.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
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
pub(crate) unsafe fn destack_fs_fgetxattr(
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch through the raw handle lane
    unsafe { destack_fs_fgetxattr_handle(context, out, handle, name) }
}

/// Set an extended attribute by path.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_setxattr(
    context: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch by path encoding
    match path.encoding {
        PathEncoding::Bytes => unsafe {
            destack_fs_setxattr_bytes(context, path.bytes, name, value, flags)
        },
        PathEncoding::Utf16 => unsafe {
            destack_fs_setxattr_utf16(context, path.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute without following symlinks.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_lsetxattr(
    context: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch by path encoding
    match path.encoding {
        PathEncoding::Bytes => unsafe {
            destack_fs_lsetxattr_bytes(context, path.bytes, name, value, flags)
        },
        PathEncoding::Utf16 => unsafe {
            destack_fs_lsetxattr_utf16(context, path.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute by handle.
///
/// Set the requested control value on the descriptor through the native option interface.
/// The binding performs one control transaction and returns the exact host outcome without policy retries.
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
pub(crate) unsafe fn destack_fs_fsetxattr(
    context: &BindingCallContext,
    handle: FileHandle,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch through the raw handle lane
    unsafe { destack_fs_fsetxattr_handle(context, handle, name, value, flags) }
}

/// List extended attribute names by path.
///
/// List extended attribute names by path via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_listxattr(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: OsPath,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read raw names for the selected path encoding
    let mut names = std::mem::MaybeUninit::<NativeArray<NativeArray<u8>>>::uninit();
    match path.encoding {
        PathEncoding::Bytes => {
            unsafe { destack_fs_listxattr_bytes(context, names.as_mut_ptr(), path.bytes) }?;
        }
        PathEncoding::Utf16 => {
            unsafe { destack_fs_listxattr_utf16(context, names.as_mut_ptr(), path.utf16) }?;
        }
    }
    let names = unsafe { names.assume_init() };
    let names = xattr_name_strings_from_bytes(context, names)?;

    // write the decoded output
    unsafe {
        *out = names;
    }

    Ok(())
}

/// List extended attribute names without following symlinks.
///
/// List extended attribute names without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_llistxattr(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: OsPath,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read raw names for the selected path encoding
    let mut names = std::mem::MaybeUninit::<NativeArray<NativeArray<u8>>>::uninit();
    match path.encoding {
        PathEncoding::Bytes => {
            unsafe { destack_fs_llistxattr_bytes(context, names.as_mut_ptr(), path.bytes) }?;
        }
        PathEncoding::Utf16 => {
            unsafe { destack_fs_llistxattr_utf16(context, names.as_mut_ptr(), path.utf16) }?;
        }
    }
    let names = unsafe { names.assume_init() };
    let names = xattr_name_strings_from_bytes(context, names)?;

    // write the decoded output
    unsafe {
        *out = names;
    }

    Ok(())
}

/// List extended attribute names by handle.
///
/// List extended attribute names by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
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
pub(crate) unsafe fn destack_fs_flistxattr(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read raw names from the handle lane
    let mut names = std::mem::MaybeUninit::<NativeArray<NativeArray<u8>>>::uninit();
    unsafe { destack_fs_flistxattr_handle(context, names.as_mut_ptr(), handle) }?;
    let names = unsafe { names.assume_init() };
    let names = xattr_name_strings_from_bytes(context, names)?;

    // write the decoded output
    unsafe {
        *out = names;
    }

    Ok(())
}

/// Remove an extended attribute by path.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_removexattr(
    context: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch by path encoding
    match path.encoding {
        PathEncoding::Bytes => unsafe { destack_fs_removexattr_bytes(context, path.bytes, name) },
        PathEncoding::Utf16 => unsafe { destack_fs_removexattr_utf16(context, path.utf16, name) },
    }
}

/// Remove an extended attribute without following symlinks.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
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
pub(crate) unsafe fn destack_fs_lremovexattr(
    context: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch by path encoding
    match path.encoding {
        PathEncoding::Bytes => unsafe { destack_fs_lremovexattr_bytes(context, path.bytes, name) },
        PathEncoding::Utf16 => unsafe { destack_fs_lremovexattr_utf16(context, path.utf16, name) },
    }
}

/// Remove an extended attribute by handle.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
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
pub(crate) unsafe fn destack_fs_fremovexattr(
    context: &BindingCallContext,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve the string name as raw bytes
    let name = xattr_name_slice_from_string(context, name)?;

    // dispatch through the raw handle lane
    unsafe { destack_fs_fremovexattr_handle(context, handle, name) }
}
