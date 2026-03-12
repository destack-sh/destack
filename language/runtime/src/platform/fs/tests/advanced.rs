#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{
    FsHarnessContext, assert_platform_error_codes_with_privileged_policy, temp_dir,
    with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    AllocFlags, FileAdvice, FileLockFlags, FileMode, FileOffset, FileSize, OpenFlags, SeekWhence,
    SyncFlags,
};
#[cfg(windows)]
use crate::platform::fs::{FdFlags, SpliceCursor, SpliceCursorVm, SpliceFlags, StatusFlags};
use crate::platform::resource::SocketHandle;

/// Windows-side representation for close-on-exec in fd-flag tests.
#[cfg(windows)]
const WINDOWS_FD_CLOEXEC_FLAG: u32 = 1;
/// Shared file-lock bit for exclusive locking.
const LOCK_EXCLUSIVE: u32 = 0x2;
/// Shared file-lock bit for nonblocking acquisition.
const LOCK_NONBLOCK: u32 = 0x4;
/// Shared file-lock bit for unlocking.
const LOCK_UNLOCK: u32 = 0x8;
/// One unknown file-lock bit used for validation tests.
const LOCK_UNKNOWN: u32 = 0x10;
/// Sync flag bit for waiting before queued writeback.
const SYNC_FILE_RANGE_WAIT_BEFORE: u32 = 0x1;
/// Sync flag bit for starting writeback.
const SYNC_FILE_RANGE_WRITE: u32 = 0x2;
/// Sync flag bit for waiting after queued writeback.
const SYNC_FILE_RANGE_WAIT_AFTER: u32 = 0x4;
/// Payload length used to force multi-chunk sendfile transfers.
const SENDFILE_STRESS_LENGTH: usize = 256 * 1024 + 137;

/// Read an exact number of bytes from one socket.
fn read_socket_exact(
    context: &mut FsHarnessContext<'_>,
    handle: SocketHandle,
    len: usize,
) -> RuntimeResult<Vec<u8>> {
    // allocate one destination buffer for the full transfer
    let mut buffer = vec![0u8; len];
    let mut filled = 0usize;

    while filled < len {
        // keep reading until the requested length is satisfied
        let read = context.socket_read(handle, &mut buffer[filled..])?;
        assert!(
            read != 0,
            "socket closed before receiving the expected payload"
        );
        filled += read as usize;
    }

    Ok(buffer)
}

/// Seek within files, lock ranges, and duplicate descriptors.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_seek_dup_lock() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_seek_dup");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // open file and write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"seek")?;
        context.destack_fs_write(handle, payload)?;

        // seek and verify offset
        let offset = context.destack_fs_seek(handle, FileOffset(0), SeekWhence::End)?;
        assert_eq!(offset.0, 4);

        // exercise file locking
        let lock = FileLockFlags(LOCK_EXCLUSIVE | LOCK_NONBLOCK);
        context.destack_fs_lock(handle, lock)?;
        let unlock = FileLockFlags(0x8);
        context.destack_fs_lock(handle, unlock)?;

        // duplicate handles
        let dup_handle = context.destack_fs_dup(handle)?;
        let target_path = context.path_bytes(&file_path);
        let target = context.destack_fs_open(
            target_path,
            OpenFlags(libc::O_RDONLY as u32),
            FileMode(0o644),
        )?;
        let dup2_handle = context.destack_fs_dup2(dup_handle, target)?;
        assert_eq!(dup2_handle.0, target.0);
        context.destack_fs_close(dup2_handle)?;

        let target_path = context.path_bytes(&file_path);
        let target = context.destack_fs_open(
            target_path,
            OpenFlags(libc::O_RDONLY as u32),
            FileMode(0o644),
        )?;
        #[cfg(unix)]
        let dup3_flags = OpenFlags(libc::O_CLOEXEC as u32);
        #[cfg(windows)]
        let dup3_flags = OpenFlags(0);
        let dup3_handle = context.destack_fs_dup3(dup_handle, target, dup3_flags)?;
        assert_eq!(dup3_handle.0, target.0);
        let fd_flags = context.destack_fs_get_fd_flags(dup3_handle)?;
        #[cfg(unix)]
        assert_ne!(fd_flags.0 & libc::FD_CLOEXEC as u32, 0);
        #[cfg(windows)]
        assert_ne!(fd_flags.0 & WINDOWS_FD_CLOEXEC_FLAG, 0);
        context.destack_fs_close(dup3_handle)?;

        // read from the duplicated handle
        let buffer = context.zeroed_bytes_slice_value(8)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_pread(dup_handle, buffer_call, FileOffset(0))?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"seek");

        // cleanup
        context.destack_fs_close(dup_handle)?;
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject invalid file-lock flag combinations instead of leaving them to host quirks.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_lock_rejects_invalid_flag_combinations() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_lock_flags");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create one file handle for lock probes
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        // reject missing lock mode
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_lock(handle, FileLockFlags(LOCK_NONBLOCK)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // reject conflicting lock modes
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_lock(handle, FileLockFlags(0x1 | LOCK_EXCLUSIVE)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // reject unlock mixed with acquisition flags
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_lock(handle, FileLockFlags(LOCK_UNLOCK | LOCK_EXCLUSIVE)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // reject unknown flag bits
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_lock(handle, FileLockFlags(LOCK_UNKNOWN)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Copy file ranges between two open file handles.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_copy_file_range() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_copy_range");
        let src_path = temp_dir.join("src.txt");
        let dst_path = temp_dir.join("dst.txt");
        let payload: Vec<u8> = (0..(256 * 1024 + 137))
            .map(|index| (index % 251) as u8)
            .collect();

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let src = context.path_bytes(&src_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let src_handle = context.destack_fs_open(src, flags, FileMode(0o644))?;
        let payload_slice = context.bytes_slice_value(&payload)?;
        context.destack_fs_write(src_handle, payload_slice)?;

        let dst = context.path_bytes(&dst_path);
        let dst_handle = context.destack_fs_open(dst, flags, FileMode(0o644))?;

        let copied = context.destack_fs_copy_file_range(
            src_handle,
            FileOffset(0),
            dst_handle,
            FileOffset(0),
            FileSize(payload.len() as u64),
        )?;
        assert_eq!(copied, payload.len() as u64);

        // copied bytes should match the full source payload
        let buffer = context.zeroed_bytes_slice_value(payload.len())?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_pread(dst_handle, buffer_call, FileOffset(0))?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, payload.as_slice());

        context.destack_fs_close(src_handle)?;
        context.destack_fs_close(dst_handle)?;
        let dst = context.path_bytes(&dst_path);
        context.destack_fs_unlink(dst)?;
        let src = context.path_bytes(&src_path);
        context.destack_fs_unlink(src)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Send file contents into a socket with sendfile.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_sendfile() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_sendfile");
        let file_path = temp_dir.join("payload.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = b"sendfile";
        let payload_bytes = context.bytes_slice_value(payload)?;
        context.destack_fs_write(handle, payload_bytes)?;

        // build a connected socket pair
        let (server, client) = context.tcp_pair()?;

        let sent = context.destack_fs_sendfile(
            server,
            handle,
            FileOffset(0),
            FileSize(payload.len() as u64),
        )?;
        assert_eq!(sent, payload.len() as u64);

        // the socket payload should match the file payload
        let mut buffer = vec![0u8; payload.len()];
        let out = context.socket_read(client, &mut buffer)?;
        buffer.truncate(out as usize);
        assert_eq!(buffer, payload);

        context.close_socket(server)?;
        context.close_socket(client)?;
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Send one large file slice into a socket without disturbing the file cursor.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_sendfile_respects_offset_and_file_cursor() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_sendfile_offset");
        let file_path = temp_dir.join("payload.txt");
        let prefix = b"prefix:";
        let suffix = b":suffix";
        let payload: Vec<u8> = (0..SENDFILE_STRESS_LENGTH)
            .map(|index| (index % 251) as u8)
            .collect();
        let mut file_bytes = Vec::with_capacity(prefix.len() + payload.len() + suffix.len());
        file_bytes.extend_from_slice(prefix);
        file_bytes.extend_from_slice(&payload);
        file_bytes.extend_from_slice(suffix);

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // open the file and write the full payload once
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let file_bytes_value = context.bytes_slice_value(&file_bytes)?;
        context.destack_fs_write(handle, file_bytes_value)?;

        // the explicit sendfile offset should not consume the file cursor
        let file_end = context.destack_fs_seek(handle, FileOffset(0), SeekWhence::Cur)?;
        assert_eq!(file_end.0, file_bytes.len() as i64);

        // build a connected socket pair and send only the interior payload bytes
        let (server, client) = context.tcp_pair()?;
        let sent = context.destack_fs_sendfile(
            server,
            handle,
            FileOffset(prefix.len() as i64),
            FileSize(payload.len() as u64),
        )?;
        assert_eq!(sent, payload.len() as u64);

        // the socket payload should match the requested slice exactly
        let received = read_socket_exact(&mut context, client, payload.len())?;
        assert_eq!(received, payload);

        // explicit offsets should leave the file cursor unchanged
        let file_end_after = context.destack_fs_seek(handle, FileOffset(0), SeekWhence::Cur)?;
        assert_eq!(file_end_after.0, file_end.0);

        // cleanup
        context.close_socket(server)?;
        context.close_socket(client)?;
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Exercise sync, allocation, and advice operations on one file handle.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_sync_alloc_advice() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_sync_alloc");
        let file_path = temp_dir.join("data.bin");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"alloc")?;
        context.destack_fs_write(handle, payload)?;

        // fdatasync should be a real flush path on current unix and windows backends
        context.destack_fs_fdatasync(handle)?;

        // sync_file_range should succeed through the host path or the explicit fallback
        context.destack_fs_sync_file_range(
            handle,
            FileOffset(0),
            FileSize(4096),
            SyncFlags(
                SYNC_FILE_RANGE_WAIT_BEFORE | SYNC_FILE_RANGE_WRITE | SYNC_FILE_RANGE_WAIT_AFTER,
            ),
        )?;

        // fallocate currently exists on linux, android, apple, and windows backends
        #[cfg(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios",
            windows
        ))]
        context.destack_fs_fallocate(handle, FileOffset(0), FileSize(4096), AllocFlags(0))?;

        #[cfg(any(
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_fallocate(handle, FileOffset(0), FileSize(4096), AllocFlags(0)),
            &[PlatformErrorCode::NotSupported],
        )?;

        // fadvise is real on linux, android, and apple, and explicitly unsupported on windows and bsd
        #[cfg(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios"
        ))]
        context.destack_fs_fadvise(
            handle,
            FileOffset(0),
            FileSize(4096),
            FileAdvice::Sequential,
        )?;

        #[cfg(any(
            windows,
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_fadvise(
                handle,
                FileOffset(0),
                FileSize(4096),
                FileAdvice::Sequential,
            ),
            &[PlatformErrorCode::NotSupported],
        )?;

        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Flush one file-backed handle through syncfs on windows.
#[cfg(windows)]
#[test]
fn test_fs_syncfs_windows_flushes_file_handle() {
    with_harness_context(|mut context| {
        // create one temp directory and file
        let temp_dir = temp_dir("fs_syncfs_windows");
        let file_path = temp_dir.join("flush.bin");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // open one writable file and write payload bytes
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"syncfs")?;
        context.destack_fs_write(handle, payload)?;

        // flush through syncfs
        context.destack_fs_syncfs(handle)?;

        // close and clean up
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Copy bytes between two file handles through splice fallback on windows.
#[cfg(windows)]
#[test]
fn test_fs_splice_windows_file_copy_fallback() {
    with_harness_context(|mut context| {
        // create one temp directory and two files
        let temp_dir = temp_dir("fs_splice_windows");
        let source_path = temp_dir.join("source.bin");
        let target_path = temp_dir.join("target.bin");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // open both files and seed source payload
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let source_path_ref = context.path_bytes(&source_path);
        let source = context.destack_fs_open(source_path_ref, flags, FileMode(0o644))?;
        let target_path_ref = context.path_bytes(&target_path);
        let target = context.destack_fs_open(target_path_ref, flags, FileMode(0o644))?;
        let payload = b"splice-fallback";
        context.destack_fs_write(source, context.bytes_slice_value(payload)?)?;

        // splice from source offset zero into target offset zero
        let source_cursor = if context.vm_context.is_some() {
            context.harness_value_vm(SpliceCursorVm {
                offset: Some(FileOffset(0)),
            })
        } else {
            context.harness_value(SpliceCursor {
                offset: Some(FileOffset(0)),
            })
        };
        let target_cursor = if context.vm_context.is_some() {
            context.harness_value_vm(SpliceCursorVm {
                offset: Some(FileOffset(0)),
            })
        } else {
            context.harness_value(SpliceCursor {
                offset: Some(FileOffset(0)),
            })
        };
        let copied = context.destack_fs_splice(
            source.0,
            source_cursor,
            target.0,
            target_cursor,
            FileSize(payload.len() as u64),
            SpliceFlags(0),
        )?;
        assert_eq!(copied, payload.len() as u64);

        // verify target payload bytes
        let buffer = context.zeroed_bytes_slice_value(payload.len())?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let read = context.destack_fs_pread(target, buffer_call, FileOffset(0))?;
        let bytes = context.bytes_prefix_from_slice_value(buffer_value, read as usize)?;
        assert_eq!(bytes, payload);

        // close and clean up
        context.destack_fs_close(target)?;
        context.destack_fs_close(source)?;
        let target_path_ref = context.path_bytes(&target_path);
        context.destack_fs_unlink(target_path_ref)?;
        let source_path_ref = context.path_bytes(&source_path);
        context.destack_fs_unlink(source_path_ref)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Duplicate one directory handle through dirfd and roundtrip fd flags on windows.
#[cfg(windows)]
#[test]
fn test_fs_dirfd_and_fd_flags_windows_roundtrip() {
    with_harness_context(|mut context| {
        // create one temp directory and open it
        let temp_dir = temp_dir("fs_dirfd_windows");
        let dir_path = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir_path, FileMode(0o755))?;

        let dir_path = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir_path)?;

        // extract one file-handle style descriptor for the directory
        let file = context.destack_fs_dirfd(directory)?;

        // verify close-on-exec is set by default
        let fd_flags = context.destack_fs_get_fd_flags(file)?;
        assert_ne!(fd_flags.0 & WINDOWS_FD_CLOEXEC_FLAG, 0);

        // clear close-on-exec and verify the update
        context.destack_fs_set_fd_flags(file, FdFlags(0))?;
        let fd_flags = context.destack_fs_get_fd_flags(file)?;
        assert_eq!(fd_flags.0 & WINDOWS_FD_CLOEXEC_FLAG, 0);

        // restore close-on-exec and verify the update
        context.destack_fs_set_fd_flags(file, FdFlags(WINDOWS_FD_CLOEXEC_FLAG))?;
        let fd_flags = context.destack_fs_get_fd_flags(file)?;
        assert_ne!(fd_flags.0 & WINDOWS_FD_CLOEXEC_FLAG, 0);

        // set status flags and verify the tracked flag roundtrip
        context.destack_fs_set_status_flags(file, StatusFlags(libc::O_APPEND as u32))?;
        let status_flags = context.destack_fs_get_status_flags(file)?;
        assert_ne!(status_flags.0 & libc::O_APPEND as u32, 0);

        // close handles and clean up
        context.destack_fs_close(file)?;
        context.destack_fs_closedir(directory)?;
        let dir_path = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir_path)?;

        Ok(())
    });
}

/// Apply append status flags through real write behavior on windows.
#[cfg(windows)]
#[test]
fn test_fs_status_flags_windows_append_behavior() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_append_windows");
        let file_path = temp_dir.join("append.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // open one file with append enabled from the start
        let file = context.path_bytes(&file_path);
        let flags =
            OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC | libc::O_APPEND) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        let status_flags = context.destack_fs_get_status_flags(handle)?;
        assert_ne!(status_flags.0 & libc::O_APPEND as u32, 0);

        // append twice even after seeking back to the start
        let buffer = context.bytes_slice_value(b"a")?;
        context.destack_fs_write(handle, buffer)?;

        let _ = context.destack_fs_seek(handle, FileOffset(0), SeekWhence::Set)?;
        let buffer = context.bytes_slice_value(b"b")?;
        context.destack_fs_write(handle, buffer)?;

        let buffer = context.zeroed_bytes_slice_value(8)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_pread(handle, buffer_call, FileOffset(0))?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"ab");

        // clear append mode and verify writes honor the explicit cursor again
        context.destack_fs_set_status_flags(handle, StatusFlags(0))?;
        let status_flags = context.destack_fs_get_status_flags(handle)?;
        assert_eq!(status_flags.0 & libc::O_APPEND as u32, 0);

        let _ = context.destack_fs_seek(handle, FileOffset(0), SeekWhence::Set)?;
        let buffer = context.bytes_slice_value(b"c")?;
        context.destack_fs_write(handle, buffer)?;

        let buffer = context.zeroed_bytes_slice_value(8)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_pread(handle, buffer_call, FileOffset(0))?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"cb");

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Keep append only opens from silently granting read access.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_append_only_open_does_not_allow_read() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_append_only_windows");
        let file_path = temp_dir.join("append_only.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // seed the file so append opens target an existing entry
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"seed")?;
        context.destack_fs_write(handle, payload)?;
        context.destack_fs_close(handle)?;

        // reopen in append only mode and verify writes still succeed
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_APPEND) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0))?;
        let payload = context.bytes_slice_value(b"+")?;
        context.destack_fs_write(handle, payload)?;

        // reject sequential reads through an append only handle
        let buffer = context.zeroed_bytes_slice_value(8)?;
        let result = context.destack_fs_read(handle, buffer);
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoInvalidData,
            ],
        )?;

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
