use super::{assert_platform_error_codes_with_privileged_policy, temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags, OpenFlags,
};

/// Memory protection bit for readable mappings.
const PROT_READ: u32 = 0x1;
/// Memory protection bit for writable mappings.
const PROT_WRITE: u32 = 0x2;
/// Mapping flag bit for shared mappings.
const MAP_SHARED: u32 = 0x1;
/// Mapping flag bit for private mappings.
const MAP_PRIVATE: u32 = 0x2;
/// Mapping flag bit for anonymous mappings.
const MAP_ANON: u32 = 0x20;
/// Sync flag bit for asynchronous msync.
const MS_ASYNC: u32 = 0x1;

/// Map anonymous memory and exercise mapping control operations.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mmap_anonymous() {
    with_harness_context(|mut context| {
        // runtime setup
        let prot = MmapProt(PROT_READ | PROT_WRITE);
        let flags = MmapFlags(MAP_PRIVATE | MAP_ANON);

        let mapping = context.destack_fs_mmap_anonymous(FileSize(4096), prot, flags)?;
        let mapping = context.mapping_from_value(mapping);

        // the anonymous mapping should roundtrip written bytes
        context.write_mapping(&mapping, b"anon")?;
        let bytes = context.read_mapping(&mapping, 4)?;
        assert_eq!(bytes, b"anon");

        context.destack_fs_mprotect(context.mapping_value(mapping), MmapProt(PROT_READ))?;
        context.destack_fs_madvise(context.mapping_value(mapping), MmapAdvice::Sequential)?;
        context.destack_fs_msync(context.mapping_value(mapping), MmapSyncFlags(MS_ASYNC))?;
        context.destack_fs_munmap(context.mapping_value(mapping))?;

        Ok(())
    });
}

/// Map file-backed memory and persist modified mapping contents.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mmap_file() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mmap_file");
        let file_path = temp_dir.join("mapping.bin");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"mmap")?;
        context.destack_fs_write(handle, payload)?;

        let prot = MmapProt(PROT_READ | PROT_WRITE);
        let flags = MmapFlags(MAP_SHARED);
        let mapping =
            context.destack_fs_mmap_file(handle, FileOffset(0), FileSize(4), prot, flags)?;
        let mapping = context.mapping_from_value(mapping);

        // mapped bytes should be readable and writable
        let bytes = context.read_mapping(&mapping, 4)?;
        assert_eq!(bytes, b"mmap");

        context.write_mapping(&mapping, b"map2")?;
        context.destack_fs_msync(context.mapping_value(mapping), MmapSyncFlags(MS_ASYNC))?;
        context.destack_fs_munmap(context.mapping_value(mapping))?;

        // file contents should reflect the synced mapping write
        let buffer = context.zeroed_bytes_slice_value(4)?;
        let (buffer_call, buffer_value) = context.duplicate_value(buffer);
        let out = context.destack_fs_pread(handle, buffer_call, FileOffset(0))?;
        let bytes = context.bytes_prefix_from_slice_value(buffer_value, out as usize)?;
        assert_eq!(bytes, b"map2");

        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}

/// Reject under-specified or contradictory mmap flag payloads.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mmap_rejects_invalid_flags() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mmap_invalid_flags");
        let file_path = temp_dir.join("mapping.bin");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;

        // create one file handle for file-backed mapping probes
        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.destack_fs_open(file, flags, FileMode(0o644))?;
        let payload = context.bytes_slice_value(b"mmap")?;
        context.destack_fs_write(handle, payload)?;

        // reject missing sharing mode for file mappings
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_mmap_file(
                handle,
                FileOffset(0),
                FileSize(4),
                MmapProt(PROT_READ),
                MmapFlags(0),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // reject anonymous flag on file mappings
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_mmap_file(
                handle,
                FileOffset(0),
                FileSize(4),
                MmapProt(PROT_READ),
                MmapFlags(MAP_SHARED | MAP_ANON),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // reject missing anonymous flag for anonymous mappings
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_mmap_anonymous(
                FileSize(4096),
                MmapProt(PROT_READ),
                MmapFlags(MAP_PRIVATE),
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // reject zero-length mappings instead of deferring to host-specific failures
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_mmap_anonymous(
                FileSize(0),
                MmapProt(PROT_READ),
                MmapFlags(MAP_PRIVATE | MAP_ANON),
            ),
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
