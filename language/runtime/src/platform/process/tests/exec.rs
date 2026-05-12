use super::{assert_platform_error_code_with_privileged_policy, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::ExecAtFlags;
use crate::platform::resource::{DirectoryHandle, FileHandle, ResourceId};

/// Report specific errors for missing executables and invalid exec handles.
#[cfg(unix)]
#[test]
fn test_process_exec_error_paths() {
    with_harness_context(|mut context| {
        let empty = Vec::<String>::new();
        let empty_values = context.string_slice_value(&empty)?;

        // missing executable path should map to process-not-found
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_exec(
                context.path_value("/definitely/missing/destack-command")?,
                empty_values,
                context.string_slice_value(&empty)?,
            ),
            PlatformErrorCode::ProcessNotFound,
        )?;

        let missing_directory = DirectoryHandle(ResourceId::local(0));
        let missing_file = FileHandle(ResourceId::local(0));

        // invalid handles should map to invalid-argument errors
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_execat(
                missing_directory,
                context.path_value("missing")?,
                context.string_slice_value(&empty)?,
                context.string_slice_value(&empty)?,
                ExecAtFlags(0),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        assert_platform_error_code_with_privileged_policy(
            context.destack_process_fexec(
                missing_file,
                context.string_slice_value(&empty)?,
                context.string_slice_value(&empty)?,
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}
