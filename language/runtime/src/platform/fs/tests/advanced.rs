#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    AllocFlags, FileAdvice, FileLockFlags, FileMode, FileOffset, FileSize, OpenFlags, SeekWhence,
    SyncFlags,
};
#[cfg(windows)]
use crate::platform::fs::{FdFlags, SpliceCursor, SpliceCursorVm, SpliceFlags, StatusFlags};

/// Windows-side representation for close-on-exec in fd-flag tests.
#[cfg(windows)]
const WINDOWS_FD_CLOEXEC_FLAG: u32 = 1;

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

        // exercise file locking if available
        let allowed = [PlatformErrorCode::NotSupported];
        let lock = FileLockFlags(0x2 | 0x4);
        let lock_result = context.destack_fs_lock(handle, lock);
        let _ = context.result_ok_or_codes(lock_result, "lock", &allowed)?;
        let unlock = FileLockFlags(0x8);
        let unlock_result = context.destack_fs_lock(handle, unlock);
        let _ = context.result_ok_or_codes(unlock_result, "unlock", &allowed)?;

        // duplicate handles
        let dup_handle = context.destack_fs_dup(handle)?;
        let target_path = context.path_bytes(&file_path);
        let target = context.destack_fs_open(
            target_path,
            OpenFlags(libc::O_RDONLY as u32),
            FileMode(0o644),
        )?;
        let dup2_result = context.destack_fs_dup2(dup_handle, target);
        let dup2 = context.result_ok_or_codes(dup2_result, "dup2", &allowed)?;
        if let Some(dup2_handle) = dup2 {
            context.destack_fs_close(dup2_handle)?;
        } else {
            context.destack_fs_close(target)?;
        }

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
        let dup3_result = context.destack_fs_dup3(dup_handle, target, dup3_flags);
        let dup3 = context.result_ok_or_codes(dup3_result, "dup3", &allowed)?;
        if let Some(dup3_handle) = dup3 {
            context.destack_fs_close(dup3_handle)?;
        } else {
            context.destack_fs_close(target)?;
        }

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

/// Copy file ranges between two open file handles.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_copy_file_range() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_copy_range");
        let src_path = temp_dir.join("src.txt");
        let dst_path = temp_dir.join("dst.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let src = context.path_bytes(&src_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let src_handle = context.destack_fs_open(src, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"copy-range")?;
        context.destack_fs_write(src_handle, payload)?;

        let dst = context.path_bytes(&dst_path);
        let dst_handle = context.destack_fs_open(dst, flags, FileMode(0o644))?;

        let allowed = [PlatformErrorCode::NotSupported];
        let copy_result = context.destack_fs_copy_file_range(
            src_handle,
            FileOffset(0),
            dst_handle,
            FileOffset(0),
            FileSize(10),
        );
        let copied = context.result_ok_or_codes(copy_result, "copy_file_range", &allowed)?;

        // when supported, copied bytes should match the source payload
        if copied.is_some() {
            let buffer = context.zeroed_bytes_slice_value(16)?;
            let (buffer_call, buffer_value) = context.duplicate_value(buffer);
            let out = context.destack_fs_pread(dst_handle, buffer_call, FileOffset(0))?;
            let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
            assert_eq!(buffer, b"copy-range");
        }

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
#[cfg(unix)]
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

        let allowed = [PlatformErrorCode::NotSupported];
        let send_result = context.destack_fs_sendfile(
            server,
            handle,
            FileOffset(0),
            FileSize(payload.len() as u64),
        );
        let sent = context.result_ok_or_codes(send_result, "sendfile", &allowed)?;

        // when supported, the socket payload should match the file payload
        if sent.is_some() {
            let mut buffer = vec![0u8; payload.len()];
            let out = context.socket_read(client, &mut buffer)?;
            buffer.truncate(out as usize);
            assert_eq!(buffer, payload);
        }

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
#[cfg(unix)]
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

        let allowed = [PlatformErrorCode::NotSupported];
        let fdatasync_result = context.destack_fs_fdatasync(handle);
        let _ = context.result_ok_or_codes(fdatasync_result, "fdatasync", &allowed)?;

        let sync_result = context.destack_fs_sync_file_range(
            handle,
            FileOffset(0),
            FileSize(4096),
            SyncFlags(0x1 | 0x2 | 0x4),
        );
        let _ = context.result_ok_or_codes(sync_result, "sync_file_range", &allowed)?;

        let fallocate_result =
            context.destack_fs_fallocate(handle, FileOffset(0), FileSize(4096), AllocFlags(0));
        let _ = context.result_ok_or_codes(fallocate_result, "fallocate", &allowed)?;

        let fadvise_result = context.destack_fs_fadvise(
            handle,
            FileOffset(0),
            FileSize(4096),
            FileAdvice::Sequential,
        );
        let _ = context.result_ok_or_codes(fadvise_result, "fadvise", &allowed)?;

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
                has_offset: true,
                offset: FileOffset(0),
            })
        } else {
            context.harness_value(SpliceCursor {
                has_offset: true,
                offset: FileOffset(0),
            })
        };
        let target_cursor = if context.vm_context.is_some() {
            context.harness_value_vm(SpliceCursorVm {
                has_offset: true,
                offset: FileOffset(0),
            })
        } else {
            context.harness_value(SpliceCursor {
                has_offset: true,
                offset: FileOffset(0),
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

        // set status flags and verify the metadata roundtrip
        context.destack_fs_set_status_flags(file, StatusFlags(0x24))?;
        let status_flags = context.destack_fs_get_status_flags(file)?;
        assert_eq!(status_flags.0, 0x24);

        // close handles and clean up
        context.destack_fs_close(file)?;
        context.destack_fs_closedir(directory)?;
        let dir_path = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir_path)?;

        Ok(())
    });
}
