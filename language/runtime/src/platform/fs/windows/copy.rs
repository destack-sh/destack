use windows_sys::Win32::Storage::FileSystem::CopyFileW;

use super::io::{destack_fs_pread, destack_fs_pwrite};
use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{CopyFlags, FileHandle, FileOffset, FileSize, PathBytes, PathUtf16};
use crate::platform::{NativeSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Copy a file with byte paths.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    _context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // decode the source and destination paths
    let from = wide_from_bytes(from, "from")?;
    let to = wide_from_bytes(to, "to")?;

    // map flags into win32 behavior
    let fail_if_exists = flags.0 & 1 != 0;

    // copy the file
    let rc = unsafe { CopyFileW(from.as_ptr(), to.as_ptr(), fail_if_exists as i32) };
    if rc == 0 {
        return Err(last_os_error("CopyFileW", None));
    }

    Ok(())
}

/// Copy a file with UTF-16 paths.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    _context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // decode the source and destination paths
    let from = wide_from_utf16(from, "from")?;
    let to = wide_from_utf16(to, "to")?;

    // map flags into win32 behavior
    let fail_if_exists = flags.0 & 1 != 0;

    // copy the file
    let rc = unsafe { CopyFileW(from.as_ptr(), to.as_ptr(), fail_if_exists as i32) };
    if rc == 0 {
        return Err(last_os_error("CopyFileW", None));
    }

    Ok(())
}

/// Copy a file range between handles.
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

    // copy via pread/pwrite to keep offsets explicit
    let mut remaining = length.0;
    let mut total = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut src_offset = src_offset;
    let mut dst_offset = dst_offset;

    // copy chunks until the source is drained or the target range is exhausted
    while remaining > 0 {
        let chunk = remaining.min(buffer.len() as u64) as usize;
        let buffer_slice = NativeSlice {
            data: buffer.as_mut_ptr(),
            len: chunk as u32,
        };
        let mut bytes_read = 0u64;
        unsafe { destack_fs_pread(context, &mut bytes_read, src, buffer_slice, src_offset) }?;
        if bytes_read == 0 {
            break;
        }
        let read_slice = NativeSlice {
            data: buffer.as_mut_ptr(),
            len: bytes_read as u32,
        };
        let mut bytes_written = 0u64;
        unsafe { destack_fs_pwrite(context, &mut bytes_written, dst, read_slice, dst_offset) }?;
        total = total.saturating_add(bytes_written);

        // advance source and destination offsets
        let written = i64::try_from(bytes_written).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "length",
                "copy size exceeds signed offset range",
            ))
            .boxed()
        })?;
        src_offset = FileOffset(src_offset.0.saturating_add(written));
        dst_offset = FileOffset(dst_offset.0.saturating_add(written));

        // advance remaining count
        remaining = remaining.saturating_sub(bytes_written);
        if bytes_written < bytes_read {
            break;
        }
    }

    unsafe {
        *out = total;
    }

    Ok(())
}
