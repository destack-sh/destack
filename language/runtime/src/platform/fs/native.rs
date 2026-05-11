use crate::diagnostic::RuntimeResult;
use crate::platform::NativeArray;
use crate::platform::abi::NativeSlice;
use crate::platform::fs::{OsPath, XattrFlags};
use crate::platform::resource::FileHandle;
use crate::runtime::BindingCallContext;

pub(crate) use super::host::*;

use super::host as host_fs;

/// Read an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_getxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_getxattr_bytes(binding, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_getxattr_utf16(binding, out, path_utf16.utf16, name)
        },
    }
}

/// Read an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lgetxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lgetxattr_bytes(binding, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lgetxattr_utf16(binding, out, path_utf16.utf16, name)
        },
    }
}

/// Set an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_setxattr_bytes(
    binding: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_setxattr_bytes(binding, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_setxattr_utf16(binding, path_utf16.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lsetxattr_bytes(
    binding: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lsetxattr_bytes(binding, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lsetxattr_utf16(binding, path_utf16.utf16, name, value, flags)
        },
    }
}

/// List extended attribute names by path as raw byte payloads.
pub(crate) unsafe fn destack_fs_listxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: OsPath,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_listxattr_bytes(binding, out, path_bytes.bytes)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_listxattr_utf16(binding, out, path_utf16.utf16)
        },
    }
}

/// List extended attribute names without following symlinks as raw byte payloads.
pub(crate) unsafe fn destack_fs_llistxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: OsPath,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_llistxattr_bytes(binding, out, path_bytes.bytes)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_llistxattr_utf16(binding, out, path_utf16.utf16)
        },
    }
}

/// Remove an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_removexattr_bytes(
    binding: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_removexattr_bytes(binding, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_removexattr_utf16(binding, path_utf16.utf16, name)
        },
    }
}

/// Remove an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lremovexattr_bytes(
    binding: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lremovexattr_bytes(binding, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lremovexattr_utf16(binding, path_utf16.utf16, name)
        },
    }
}

/// Read an extended attribute by handle with a raw name payload.
pub(crate) unsafe fn destack_fs_fgetxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fgetxattr_handle(binding, out, handle, name) }
}

/// Set an extended attribute by handle with a raw name payload.
pub(crate) unsafe fn destack_fs_fsetxattr_bytes(
    binding: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fsetxattr_handle(binding, handle, name, value, flags) }
}

/// List extended attribute names by handle as raw byte payloads.
pub(crate) unsafe fn destack_fs_flistxattr_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_flistxattr_handle(binding, out, handle) }
}

/// Remove an extended attribute by handle with a raw name payload.
pub(crate) unsafe fn destack_fs_fremovexattr_bytes(
    binding: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fremovexattr_handle(binding, handle, name) }
}
