#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    AllocFlags, FileAdvice, FileLockFlags, FileMode, FileOffset, FileSize, OpenFlags, SeekWhence,
    SyncFlags,
};

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
