#![cfg_attr(windows, allow(dead_code, unused_imports))]
use std::path::Path;

use super::{assert_platform_error_codes_with_privileged_policy, temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    FileMode, FileOffset, FileSize, NodeDevice, OpenFlags, ReadWriteFlags, SeekWhence, SpliceFlags,
};
use crate::platform::resource::{PipeHandle, ResourceId};

/// Windows local value for `O_DIRECTORY`.
#[cfg(windows)]
const WINDOWS_O_DIRECTORY: u32 = 0o200000;
/// Positioned I/O flag bit for high-priority polling.
#[cfg(target_os = "linux")]
const RWF_HIPRI: u32 = 0x1;

/// Require `O_DIRECTORY` opens to accept directories and reject regular files.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_open_directory_flag_requires_directory() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_open_directory_flag");
        let file_path = temp_dir.join("data.txt");
        let child_dir = temp_dir.join("child");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        context.destack_fs_close(handle)?;

        let child = context.path_bytes(&child_dir);
        context.destack_fs_mkdir(child, FileMode(0o755))?;

        #[cfg(unix)]
        let directory_flags = OpenFlags(libc::O_DIRECTORY as u32);
        #[cfg(windows)]
        let directory_flags = OpenFlags(WINDOWS_O_DIRECTORY);

        // reject opening a regular file as a directory
        let file = context.path_bytes(&file_path);
        let result = context.destack_fs_open(file, directory_flags, FileMode(0));
        assert_platform_error_codes_with_privileged_policy(
            result,
            &[PlatformErrorCode::IoNotDirectory],
        )?;

        // allow opening a real directory with the directory flag
        let child = context.path_bytes(&child_dir);
        let handle = context.destack_fs_open(child, directory_flags, FileMode(0))?;
        context.destack_fs_close(handle)?;

        // cleanup
        let child = context.path_bytes(&child_dir);
        context.destack_fs_rmdir(child)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Truncate a file and persist the change with fsync.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_truncate_and_fsync() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_truncate");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        let buffer = b"hello world".to_vec();
        let buffer_value = context.bytes_slice_value(&buffer)?;
        context.destack_fs_pwrite(handle, buffer_value, FileOffset(0))?;
        context.destack_fs_fsync(handle)?;
        context.destack_fs_ftruncate(handle, FileOffset(5))?;

        let buffer = context.zeroed_bytes_slice_value(16)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_pread(handle, buffer_call, FileOffset(0))?;
        let buffer = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(buffer, b"hello");

        // cleanup
        context.destack_fs_close(handle)?;

        let file = context.path_bytes(&file_path);
        context.destack_fs_truncate(file, FileOffset(0))?;
        context.destack_fs_unlink(file)?;

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Write and read file data through vectored io calls.
#[cfg(unix)]
#[test]
fn test_fs_readv_writev() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_vectored");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        let buf_a = b"hello ".to_vec();
        let buf_b = b"world".to_vec();
        let buffers = [&buf_a[..], &buf_b[..]];
        let buffers = context.bytes_slices_value(&buffers)?;
        let out = context.destack_fs_writev(handle, buffers)?;
        assert_eq!(out, 11);

        let _ = context.destack_fs_seek(handle, FileOffset(0), SeekWhence::Set)?;

        let mut buffers = vec![vec![0u8; 6], vec![0u8; 5]];
        let buffers = context.mutable_bytes_slices_value(&mut buffers)?;
        let (buffers_call, buffers_value) = context.duplicate_value(buffers);
        let out = context.destack_fs_readv(handle, buffers_call)?;
        let mut combined = context.bytes_slices_from_value(buffers_value)?.concat();
        combined.truncate(out as usize);
        assert_eq!(combined, b"hello world");

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Write and read file data through positional vectored io calls.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_preadv_pwritev() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_preadv");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write payload using pwritev
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        let buf_a = b"hello ".to_vec();
        let buf_b = b"world".to_vec();
        let buffers = [&buf_a[..], &buf_b[..]];
        let buffers = context.bytes_slices_value(&buffers)?;
        let out = context.destack_fs_pwritev(handle, buffers, FileOffset(0))?;
        assert_eq!(out, 11);

        // read back using preadv
        let mut buffers = vec![vec![0u8; 6], vec![0u8; 5]];
        let buffers = context.mutable_bytes_slices_value(&mut buffers)?;
        let (buffers_call, buffers_value) = context.duplicate_value(buffers);
        let out = context.destack_fs_preadv(handle, buffers_call, FileOffset(0))?;
        let mut combined = context.bytes_slices_from_value(buffers_value)?.concat();
        combined.truncate(out as usize);
        assert_eq!(combined, b"hello world");

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Write and read file data through positional vectored io v2 calls with zero flags.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_preadv2_pwritev2_flags_zero_roundtrip() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_preadv2_zero");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // write payload using pwritev2 with zero flags
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;

        let buffer_a = b"hello ".to_vec();
        let buffer_b = b"world".to_vec();
        let write_buffers = [&buffer_a[..], &buffer_b[..]];
        let write_buffers = context.bytes_slices_value(&write_buffers)?;
        let written =
            context.destack_fs_pwritev2(handle, write_buffers, FileOffset(0), ReadWriteFlags(0))?;
        assert_eq!(written, 11);

        // read back using preadv2 with zero flags
        let mut read_buffers = vec![vec![0u8; 6], vec![0u8; 5]];
        let read_buffers = context.mutable_bytes_slices_value(&mut read_buffers)?;
        let (read_buffers_call, read_buffers_value) = context.duplicate_value(read_buffers);
        let read = context.destack_fs_preadv2(
            handle,
            read_buffers_call,
            FileOffset(0),
            ReadWriteFlags(0),
        )?;
        let mut combined = context
            .bytes_slices_from_value(read_buffers_value)?
            .concat();
        combined.truncate(read as usize);
        assert_eq!(combined, b"hello world");

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject non-zero preadv2 and pwritev2 flags on hosts without v2 flag support.
#[cfg(target_os = "linux")]
#[test]
fn test_fs_preadv2_pwritev2_nonzero_flags_support_matches_platform() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_preadv2_flags");
        let file_path = temp_dir.join("data.txt");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create one file with one payload
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"flags")?;
        context.destack_fs_write(handle, payload)?;

        // run one preadv2 probe with non-zero flags
        let mut preadv2_buffers = vec![vec![0u8; 5]];
        let preadv2_buffers = context.mutable_bytes_slices_value(&mut preadv2_buffers)?;
        let preadv2_result = context.destack_fs_preadv2(
            handle,
            preadv2_buffers,
            FileOffset(0),
            ReadWriteFlags(RWF_HIPRI),
        );

        // run one pwritev2 probe with non-zero flags
        let pwritev2_payload = b"x".to_vec();
        let pwritev2_buffers = [&pwritev2_payload[..]];
        let pwritev2_buffers = context.bytes_slices_value(&pwritev2_buffers)?;
        let pwritev2_result = context.destack_fs_pwritev2(
            handle,
            pwritev2_buffers,
            FileOffset(0),
            ReadWriteFlags(RWF_HIPRI),
        );

        // linux hosts may accept the flags or reject by kernel policy or support level
        if let Err(error) = preadv2_result {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::InvalidArgumentValue,
                    PlatformErrorCode::IoInvalidData,
                    PlatformErrorCode::IoWouldBlock,
                ],
            )?;
        }
        if let Err(error) = pwritev2_result {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::InvalidArgumentValue,
                    PlatformErrorCode::IoInvalidData,
                    PlatformErrorCode::IoWouldBlock,
                ],
            )?;
        }

        // cleanup
        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Create fifo and fifoat nodes where supported and verify fifo mode bits.
#[cfg(unix)]
#[test]
fn test_fs_mkfifo_and_mkfifoat_support_matches_platform() {
    with_harness_context(|mut context| {
        #[cfg(target_os = "linux")]
        let fifo_type_mask = libc::S_IFMT;
        #[cfg(not(target_os = "linux"))]
        let fifo_type_mask = libc::S_IFMT as u32;

        #[cfg(target_os = "linux")]
        let fifo_type_value = libc::S_IFIFO;
        #[cfg(not(target_os = "linux"))]
        let fifo_type_value = libc::S_IFIFO as u32;

        // runtime and temp directory
        let temp_dir = temp_dir("fs_mkfifo");
        let fifo_path = temp_dir.join("fifo.pipe");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create fifo through absolute path lane
        let fifo = context.path_bytes(&fifo_path);
        let mkfifo_result = context.destack_fs_mkfifo(fifo, FileMode(0o644));

        // enforce platform behavior for mkfifo
        mkfifo_result?;
        let fifo = context.path_bytes(&fifo_path);
        let stat = context.destack_fs_stat(fifo)?;
        assert_eq!(stat.mode.0 & fifo_type_mask, fifo_type_value);

        // create fifo through directory-relative lane
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        let fifo_relative = context.path_bytes(Path::new("fifoat.pipe"));
        let mkfifoat_result =
            context.destack_fs_mkfifoat(directory, fifo_relative, FileMode(0o644));

        // enforce platform behavior for mkfifoat
        mkfifoat_result?;
        let fifo = context.path_bytes(&temp_dir.join("fifoat.pipe"));
        let stat = context.destack_fs_stat(fifo)?;
        assert_eq!(stat.mode.0 & fifo_type_mask, fifo_type_value);

        // cleanup created nodes and directory resources
        context.destack_fs_closedir(directory)?;
        let fifo = context.path_bytes(&temp_dir.join("fifoat.pipe"));
        context.destack_fs_unlink(fifo)?;
        let fifo = context.path_bytes(&fifo_path);
        context.destack_fs_unlink(fifo)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Create fifo nodes through utf16 path lanes on unix hosts.
#[cfg(unix)]
#[test]
fn test_fs_mkfifo_and_mkfifoat_utf16_roundtrip() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mkfifo_utf16");
        let fifo_path = temp_dir.join("fifo.pipe");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create fifo through absolute utf16 path lane
        let fifo = context.path_utf16(&fifo_path);
        context.destack_fs_mkfifo(fifo, FileMode(0o644))?;
        let fifo = context.path_bytes(&fifo_path);
        let stat = context.destack_fs_stat(fifo)?;
        assert_eq!(stat.mode.0 & libc::S_IFMT as u32, libc::S_IFIFO as u32);

        // create fifo through directory-relative utf16 lane
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        let fifo_relative = context.path_utf16(Path::new("fifoat.pipe"));
        context.destack_fs_mkfifoat(directory, fifo_relative, FileMode(0o644))?;
        let fifo = context.path_bytes(&temp_dir.join("fifoat.pipe"));
        let stat = context.destack_fs_stat(fifo)?;
        assert_eq!(stat.mode.0 & libc::S_IFMT as u32, libc::S_IFIFO as u32);

        // cleanup created nodes and directory resources
        context.destack_fs_closedir(directory)?;
        let fifo = context.path_bytes(&temp_dir.join("fifoat.pipe"));
        context.destack_fs_unlink(fifo)?;
        let fifo = context.path_bytes(&fifo_path);
        context.destack_fs_unlink(fifo)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Create mknod and mknodat fifo nodes where supported and verify fifo mode bits.
#[cfg(unix)]
#[test]
fn test_fs_mknod_and_mknodat_fifo_support_matches_platform() {
    with_harness_context(|mut context| {
        #[cfg(target_os = "linux")]
        let fifo_type_mask = libc::S_IFMT;
        #[cfg(not(target_os = "linux"))]
        let fifo_type_mask = libc::S_IFMT as u32;

        #[cfg(target_os = "linux")]
        let fifo_type_value = libc::S_IFIFO;
        #[cfg(not(target_os = "linux"))]
        let fifo_type_value = libc::S_IFIFO as u32;

        // runtime and temp directory
        let temp_dir = temp_dir("fs_mknod");
        let node_path = temp_dir.join("node.pipe");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create fifo-style node through mknod
        let node_mode = FileMode(fifo_type_value | 0o644);
        let node = context.path_bytes(&node_path);
        let mknod_result = context.destack_fs_mknod(node, node_mode, NodeDevice(0));

        // enforce platform behavior for mknod fifo nodes
        if let Err(error) = mknod_result {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &[
                    PlatformErrorCode::IoPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        } else {
            let node = context.path_bytes(&node_path);
            let stat = context.destack_fs_stat(node)?;
            assert_eq!(stat.mode.0 & fifo_type_mask, fifo_type_value);
        }

        // create fifo-style node through mknodat
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        let node_relative = context.path_bytes(Path::new("nodeat.pipe"));
        let mknodat_result =
            context.destack_fs_mknodat(directory, node_relative, node_mode, NodeDevice(0));

        // enforce platform behavior for mknodat fifo nodes
        if let Err(error) = mknodat_result {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &[
                    PlatformErrorCode::IoPermissionDenied,
                    PlatformErrorCode::NotSupported,
                ],
            )?;
        } else {
            let node = context.path_bytes(&temp_dir.join("nodeat.pipe"));
            let stat = context.destack_fs_stat(node)?;
            assert_eq!(stat.mode.0 & fifo_type_mask, fifo_type_value);
        }

        // cleanup created nodes and directory resources
        context.destack_fs_closedir(directory)?;
        let nodeat_path = temp_dir.join("nodeat.pipe");
        if nodeat_path.exists() {
            let node = context.path_bytes(&nodeat_path);
            context.destack_fs_unlink(node)?;
        }

        if node_path.exists() {
            let node = context.path_bytes(&node_path);
            context.destack_fs_unlink(node)?;
        }
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Create fifo-style nodes through utf16 path lanes on unix hosts.
#[cfg(unix)]
#[test]
fn test_fs_mknod_and_mknodat_utf16_roundtrip() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mknod_utf16");
        let node_path = temp_dir.join("node.pipe");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create fifo-style node through utf16 mknod
        let node_mode = FileMode((libc::S_IFIFO as u32) | 0o644);
        let node = context.path_utf16(&node_path);
        context.destack_fs_mknod(node, node_mode, NodeDevice(0))?;
        let node = context.path_bytes(&node_path);
        let stat = context.destack_fs_stat(node)?;
        assert_eq!(stat.mode.0 & libc::S_IFMT as u32, libc::S_IFIFO as u32);

        // create fifo-style node through utf16 mknodat
        let dir = context.path_bytes(&temp_dir);
        let directory = context.destack_fs_opendir(dir)?;
        let node_relative = context.path_utf16(Path::new("nodeat.pipe"));
        context.destack_fs_mknodat(directory, node_relative, node_mode, NodeDevice(0))?;
        let node = context.path_bytes(&temp_dir.join("nodeat.pipe"));
        let stat = context.destack_fs_stat(node)?;
        assert_eq!(stat.mode.0 & libc::S_IFMT as u32, libc::S_IFIFO as u32);

        // cleanup created nodes and directory resources
        context.destack_fs_closedir(directory)?;
        let node = context.path_bytes(&temp_dir.join("nodeat.pipe"));
        context.destack_fs_unlink(node)?;
        let node = context.path_bytes(&node_path);
        context.destack_fs_unlink(node)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Surface explicit unsupported or invalid-handle outcomes for tee and vmsplice lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_tee_vmsplice_invalid_handle_contract() {
    with_harness_context(|mut context| {
        // prepare one invalid pipe handle and one payload
        let invalid_pipe = PipeHandle(ResourceId::local(0));
        let payload = [b"destack".as_slice()];
        let payload = context.bytes_slices_value(&payload)?;

        // linux validates the pipe handle before tee dispatch
        #[cfg(target_os = "linux")]
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_tee(invalid_pipe, invalid_pipe, FileSize(64), SpliceFlags(0)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // non-linux targets reject tee at the feature boundary
        #[cfg(not(target_os = "linux"))]
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_tee(invalid_pipe, invalid_pipe, FileSize(64), SpliceFlags(0)),
            &[PlatformErrorCode::NotSupported],
        )?;

        // linux validates the pipe handle before vmsplice dispatch
        #[cfg(target_os = "linux")]
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_vmsplice(invalid_pipe, payload, SpliceFlags(0)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // non-linux targets reject vmsplice at the feature boundary
        #[cfg(not(target_os = "linux"))]
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_vmsplice(invalid_pipe, payload, SpliceFlags(0)),
            &[PlatformErrorCode::NotSupported],
        )?;

        Ok(())
    });
}
