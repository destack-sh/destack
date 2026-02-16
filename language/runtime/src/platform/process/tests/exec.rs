use super::{assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::ExecAtFlags;
use crate::platform::resource::{DirectoryHandle, FileHandle, ResourceId};

/// Report specific errors for missing executables and invalid exec handles.
#[cfg(unix)]
#[test]
fn test_process_exec_error_paths() {
    with_harness_context(|mut context| {
        let empty = Vec::<String>::new();

        // missing executable path should map to process-not-found
        assert_platform_error_code(
            context.exec("/definitely/missing/destack-command", &empty, &empty),
            PlatformErrorCode::ProcessNotFound,
        )?;

        let missing_directory = DirectoryHandle(ResourceId(0));
        let missing_file = FileHandle(ResourceId(0));

        // invalid handles should map to invalid-argument errors
        assert_platform_error_code(
            context.execat(missing_directory, "missing", &empty, &empty, ExecAtFlags(0)),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        assert_platform_error_code(
            context.fexec(missing_file, &empty, &empty),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}
