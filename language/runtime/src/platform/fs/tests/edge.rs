use super::{assert_platform_error_codes_with_privileged_policy, temp_dir, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::FileMode;
use crate::platform::resource::{DirectoryHandle, FileHandle, ResourceId};

/// Reject reads and closes that use invalid file handles.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_invalid_file_handle() {
    with_harness_context(|mut context| {
        // exercise invalid file handle paths
        let buffer = context.zeroed_bytes_slice_value(16)?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_read(FileHandle(ResourceId::local(9999)), buffer),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_close(FileHandle(ResourceId::local(9999))),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        Ok(())
    });
}

/// Reject directory operations that use closed or invalid directory handles.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_invalid_directory_handle() {
    with_harness_context(|mut context| {
        // runtime and temp directory
        let temp_dir = temp_dir("fs_dir_invalid");
        let sub_dir = temp_dir.join("child");

        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_mkdir(dir, FileMode(0o755))?;
        let child = context.path_bytes(&sub_dir);
        context.destack_fs_mkdir(child, FileMode(0o755))?;

        let dir = context.path_bytes(&temp_dir);
        let handle = context.destack_fs_opendir(dir)?;
        context.destack_fs_closedir(handle)?;

        // exercise invalid directory handle paths
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_closedir(handle),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_closedir(DirectoryHandle(ResourceId::local(9999))),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let child = context.path_bytes(&sub_dir);
        context.destack_fs_rmdir(child)?;
        let dir = context.path_bytes(&temp_dir);
        context.destack_fs_rmdir(dir)?;

        Ok(())
    });
}
