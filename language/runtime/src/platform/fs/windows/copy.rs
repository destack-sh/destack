use windows_sys::Win32::Storage::FileSystem::CopyFileW;

use super::io::{destack_fs_pread, destack_fs_pwrite};
use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeSlice;
use crate::platform::fs::{
    CopyFlags, FileHandle, FileOffset, FileSize, OsPath, PathBytes, PathUtf16, core as core_fs,
};
use crate::runtime::BindingCallContext;

/// Copyfile flag to reject replacing an existing destination.
const COPYFILE_FAIL_IF_EXISTS: u32 = 0x1;

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    _binding: &BindingCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // reject unsupported copyfile flags on windows
    let supported_flags = COPYFILE_FAIL_IF_EXISTS;
    if flags.0 & !supported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported copyfile flags",
        ))
        .boxed());
    }

    // decode the source and destination paths
    let from = wide_from_bytes(from, "from")?;
    let to = wide_from_bytes(to, "to")?;

    // map flags into win32 behavior
    let fail_if_exists = flags.0 & COPYFILE_FAIL_IF_EXISTS != 0;

    // copy the file
    let rc = unsafe { CopyFileW(from.as_ptr(), to.as_ptr(), fail_if_exists as i32) };
    if rc == 0 {
        return Err(last_os_error("CopyFileW", None));
    }

    Ok(())
}

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    _binding: &BindingCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    // reject unsupported copyfile flags on windows
    let supported_flags = COPYFILE_FAIL_IF_EXISTS;
    if flags.0 & !supported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported copyfile flags",
        ))
        .boxed());
    }

    // decode the source and destination paths
    let from = wide_from_utf16(from, "from")?;
    let to = wide_from_utf16(to, "to")?;

    // map flags into win32 behavior
    let fail_if_exists = flags.0 & COPYFILE_FAIL_IF_EXISTS != 0;

    // copy the file
    let rc = unsafe { CopyFileW(from.as_ptr(), to.as_ptr(), fail_if_exists as i32) };
    if rc == 0 {
        return Err(last_os_error("CopyFileW", None));
    }

    Ok(())
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

    // copy via pread/pwrite to keep offsets explicit
    let mut remaining = length.0;
    let mut total = 0u64;
    let buffer_length = core_fs::copy_fallback_buffer_length(length.0);
    let mut buffer = vec![0u8; buffer_length];
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
        unsafe { destack_fs_pread(binding, &mut bytes_read, src, buffer_slice, src_offset) }?;
        if bytes_read == 0 {
            break;
        }
        // write the full read chunk before advancing source state
        let mut written = 0u64;
        while written < bytes_read {
            let local_offset = i64::try_from(written).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "length",
                    "copy size exceeds signed offset range",
                ))
                .boxed()
            })?;
            let read_slice = NativeSlice {
                data: unsafe { buffer.as_mut_ptr().add(written as usize) },
                len: (bytes_read - written) as u32,
            };
            let mut local = 0u64;
            unsafe {
                destack_fs_pwrite(
                    binding,
                    &mut local,
                    dst,
                    read_slice,
                    FileOffset(dst_offset.0.saturating_add(local_offset)),
                )
            }?;
            if local == 0 {
                return Err(RuntimeError::from(PlatformError::io(
                    "copy_file_range fallback write returned zero bytes".to_string(),
                ))
                .boxed());
            }
            written = written.saturating_add(local);
        }

        let chunk_length = i64::try_from(bytes_read).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "length",
                "copy size exceeds signed offset range",
            ))
            .boxed()
        })?;
        src_offset = FileOffset(src_offset.0.saturating_add(chunk_length));
        dst_offset = FileOffset(dst_offset.0.saturating_add(chunk_length));
        total = total.saturating_add(bytes_read);
        remaining = remaining.saturating_sub(bytes_read);
    }

    unsafe {
        *out = total;
    }

    Ok(())
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
