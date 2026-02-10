use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{OsPath, XattrFlags, core as core_fs};
use crate::platform::resource::FileHandle;
use crate::platform::{NativeArray, NativeSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

#[allow(unused_imports)]
pub(crate) use crate::platform::fs::os::*;
pub(crate) use core_fs::*;

/// Read an extended attribute by path with byte attribute naming.
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

/// Read an extended attribute by path without following symlinks, with byte attribute naming.
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

/// Set an extended attribute by path with byte attribute naming.
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

/// Set an extended attribute by path without following symlinks, with byte attribute naming.
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

/// List extended attribute names by path as raw bytes.
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

/// List extended attribute names by path without following symlinks, as raw bytes.
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

/// Remove an extended attribute by path with byte attribute naming.
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

/// Remove an extended attribute by path without following symlinks, with byte attribute naming.
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

/// Read an extended attribute by handle with byte-path naming.
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

/// Set an extended attribute by handle with byte-path naming.
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

/// List extended attribute names by handle with byte-path naming.
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

/// Remove an extended attribute by handle with byte-path naming.
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
