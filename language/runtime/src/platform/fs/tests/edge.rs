use super::{FsHarnessKind, temp_dir, with_harness_context};
use crate::platform::fs as platform_fs;
use crate::platform::fs::FileMode;
use crate::platform::resource::{DirectoryHandle, FileHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_fs_invalid_file_handle() {
    with_harness_context(|mut context| {
        // exercise invalid file handle paths
        let mut buffer = vec![0u8; 16];
        match context.kind() {
            FsHarnessKind::Native => {
                let slice = super::native_slice_mut(&mut buffer);
                let mut out = 0u64;
                let status = unsafe {
                    platform_fs::destack_fs_file_read(&mut out, FileHandle(ResourceId(9999)), slice)
                };
                context.status_err(status, "read invalid handle")?;

                let status =
                    unsafe { platform_fs::destack_fs_file_close(FileHandle(ResourceId(9999))) };
                context.status_err(status, "close invalid handle")?;
            }
            FsHarnessKind::Vm => {
                let result = context.read(FileHandle(ResourceId(9999)), &mut buffer);
                assert!(result.is_err(), "read invalid handle should error");
                let result = context.close(FileHandle(ResourceId(9999)));
                assert!(result.is_err(), "close invalid handle should error");
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_fs_invalid_directory_handle() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_dir_invalid");
        let sub_dir = temp_dir.join("child");

        let dir = context.path_bytes(&temp_dir);
        context.mkdir(dir, FileMode(0o755))?;
        let child = context.path_bytes(&sub_dir);
        context.mkdir(child, FileMode(0o755))?;

        let dir = context.path_bytes(&temp_dir);
        let handle = context.opendir(dir)?;
        context.closedir(handle)?;

        // exercise invalid directory handle paths
        match context.kind() {
            FsHarnessKind::Native => {
                let status = unsafe { platform_fs::destack_fs_dir_closedir(handle) };
                context.status_err(status, "closedir after close")?;
                let status = unsafe {
                    platform_fs::destack_fs_dir_closedir(DirectoryHandle(ResourceId(9999)))
                };
                context.status_err(status, "closedir invalid handle")?;
            }
            FsHarnessKind::Vm => {
                let result = context.closedir(handle);
                assert!(result.is_err(), "closedir after close should error");
                let result = context.closedir(DirectoryHandle(ResourceId(9999)));
                assert!(result.is_err(), "closedir invalid handle should error");
            }
        }

        // cleanup
        let child = context.path_bytes(&sub_dir);
        context.rmdir(child)?;
        let dir = context.path_bytes(&temp_dir);
        context.rmdir(dir)?;

        Ok(())
    });
}
