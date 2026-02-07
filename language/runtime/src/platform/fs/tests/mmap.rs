use super::{temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags, OpenFlags,
};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_mmap_anonymous() {
    with_harness_context(|mut context| {
        // runtime setup
        let allowed = [PlatformErrorCode::NotSupported];
        let prot = MmapProt(0x1 | 0x2);
        let flags = MmapFlags(0x2 | 0x20);

        let mapping = context.mmap_anonymous(FileSize(4096), prot, flags)?;

        if let Some(mapping) = mapping {
            context.write_mapping(&mapping, b"anon")?;
            let bytes = context.read_mapping(&mapping, 4)?;
            assert_eq!(bytes, b"anon");

            let protect_result = context.mprotect(mapping, MmapProt(0x1));
            let _ = context.result_ok_or_codes(protect_result, "mprotect", &allowed)?;

            let advise_result = context.madvise(mapping, MmapAdvice::Sequential);
            let _ = context.result_ok_or_codes(advise_result, "madvise", &allowed)?;

            let sync_result = context.msync(mapping, MmapSyncFlags(0x1));
            let _ = context.result_ok_or_codes(sync_result, "msync", &allowed)?;
            context.munmap(mapping)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_mmap_file() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_mmap_file");
        let file_path = temp_dir.join("mapping.bin");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;

        let file = context.path_bytes(&file_path);
        let flags = OpenFlags((libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as u32);
        let handle = context.open(file, flags, FileMode(0o644))?;
        context.write(handle, b"mmap")?;

        let allowed = [PlatformErrorCode::NotSupported];
        let prot = MmapProt(0x1 | 0x2);
        let flags = MmapFlags(0x2);
        let mapping = context.mmap_file(handle, FileOffset(0), FileSize(4), prot, flags)?;

        if let Some(mapping) = mapping {
            let bytes = context.read_mapping(&mapping, 4)?;
            assert_eq!(bytes, b"mmap");
            context.write_mapping(&mapping, b"map2")?;
            let sync_result = context.msync(mapping, MmapSyncFlags(0x1));
            let _ = context.result_ok_or_codes(sync_result, "msync", &allowed)?;
            context.munmap(mapping)?;
        }

        context.close(handle)?;
        let file = context.path_bytes(&file_path);
        context.unlink(file)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
