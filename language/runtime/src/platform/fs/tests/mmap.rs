use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags, OpenFlags,
};

/// Map anonymous memory and exercise mapping control operations.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_mmap_anonymous() {
    with_harness_context(|mut context| {
        // runtime setup
        let allowed = [PlatformErrorCode::NotSupported];
        let prot = MmapProt(0x1 | 0x2);
        let flags = MmapFlags(0x2 | 0x20);

        let mapping_result = context.destack_fs_mmap_anonymous(FileSize(4096), prot, flags);
        let mapping = context.result_ok_or_codes(mapping_result, "mmap_anonymous", &allowed)?;

        // when supported, the anonymous mapping should roundtrip written bytes
        if let Some(mapping) = mapping {
            let mapping = context.mapping_from_value(mapping);
            context.write_mapping(&mapping, b"anon")?;
            let bytes = context.read_mapping(&mapping, 4)?;
            assert_eq!(bytes, b"anon");

            let protect_result =
                context.destack_fs_mprotect(context.mapping_value(mapping), MmapProt(0x1));
            let _ = context.result_ok_or_codes(protect_result, "mprotect", &allowed)?;

            let advise_result =
                context.destack_fs_madvise(context.mapping_value(mapping), MmapAdvice::Sequential);
            let _ = context.result_ok_or_codes(advise_result, "madvise", &allowed)?;

            let sync_result =
                context.destack_fs_msync(context.mapping_value(mapping), MmapSyncFlags(0x1));
            let _ = context.result_ok_or_codes(sync_result, "msync", &allowed)?;
            context.destack_fs_munmap(context.mapping_value(mapping))?;
        }

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

        let allowed = [PlatformErrorCode::NotSupported];
        let prot = MmapProt(0x1 | 0x2);
        let flags = MmapFlags(0x2);
        let mapping_result =
            context.destack_fs_mmap_file(handle, FileOffset(0), FileSize(4), prot, flags);
        let mapping = context.result_ok_or_codes(mapping_result, "mmap_file", &allowed)?;

        // when supported, mapped bytes should be readable and writable
        if let Some(mapping) = mapping {
            let mapping = context.mapping_from_value(mapping);
            let bytes = context.read_mapping(&mapping, 4)?;
            assert_eq!(bytes, b"mmap");
            context.write_mapping(&mapping, b"map2")?;
            let sync_result =
                context.destack_fs_msync(context.mapping_value(mapping), MmapSyncFlags(0x1));
            let _ = context.result_ok_or_codes(sync_result, "msync", &allowed)?;
            context.destack_fs_munmap(context.mapping_value(mapping))?;
        }

        context.destack_fs_close(handle)?;
        let file = context.path_bytes(&file_path);
        context.destack_fs_unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
