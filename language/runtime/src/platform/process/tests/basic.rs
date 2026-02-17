use super::{assert_platform_error_code, unique_env_name, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{GroupId, UserId};

/// Read process arguments through native and vm harnesses.
#[test]
fn test_process_args_roundtrip() {
    with_harness_context(|mut context| {
        let args = context.destack_process_args()?;
        let args = context.string_list_from_value(args)?;
        assert!(args.is_empty());
        Ok(())
    });
}

/// Ensure the shared process harness executes both native and vm backends.
#[test]
fn test_process_harness_runs_native_and_vm() {
    let mut seen_native = false;
    let mut seen_vm = false;

    with_harness_context(|context| {
        if context.is_vm() {
            seen_vm = true;
        } else {
            seen_native = true;
        }

        Ok(())
    });

    // both harness variants should be exercised
    assert!(seen_native);
    assert!(seen_vm);
}

/// Open and close standard stream handles through process fd bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_process_stdio_handles_open_and_close() {
    with_harness_context(|mut context| {
        let stdin = context.destack_process_stdio_stdin()?;
        let stdout = context.destack_process_stdio_stdout()?;
        let stderr = context.destack_process_stdio_stderr()?;

        assert_ne!(stdin.0, stdout.0);
        assert_ne!(stdin.0, stderr.0);
        assert_ne!(stdout.0, stderr.0);

        context.fs_close_handle(stdin)?;
        context.fs_close_handle(stdout)?;
        context.fs_close_handle(stderr)?;

        assert_platform_error_code(
            context.fs_close_handle(stdout),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Write bytes through stdout and stderr handles from process fd bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_process_stdio_write_roundtrip() {
    with_harness_context(|mut context| {
        let stdout = context.destack_process_stdio_stdout()?;
        let stderr = context.destack_process_stdio_stderr()?;

        let stdout_payload = b"[destack-process-stdout]\n";
        let stderr_payload = b"[destack-process-stderr]\n";

        let stdout_written = context.fs_write_bytes(stdout, stdout_payload)?;
        let stderr_written = context.fs_write_bytes(stderr, stderr_payload)?;

        assert_eq!(stdout_written, stdout_payload.len() as u64);
        assert_eq!(stderr_written, stderr_payload.len() as u64);

        context.fs_close_handle(stdout)?;
        context.fs_close_handle(stderr)?;

        Ok(())
    });
}

/// Set, read, and delete one utf8 environment variable.
#[cfg(any(unix, windows))]
#[test]
fn test_process_env_roundtrip() {
    let name = unique_env_name("UTF8");
    let value = "destack-process-env";

    with_harness_context(|mut context| {
        let _ = context.destack_process_env_delete(context.string_value(&name));
        context
            .destack_process_env_set(context.string_value(&name), context.string_value(value))?;
        let value_out = context.destack_process_env_get(context.string_value(&name))?;
        let value_out = context.string_from_value(value_out)?;
        assert_eq!(value_out, value);

        context.destack_process_env_delete(context.string_value(&name))?;
        // deleted variables should report missing-variable errors
        assert_platform_error_code(
            context.destack_process_env_get(context.string_value(&name)),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Set, read, and delete one byte-oriented environment variable.
#[cfg(unix)]
#[test]
fn test_process_env_bytes_roundtrip() {
    let name = unique_env_name("BYTES").into_bytes();
    let value = vec![0xFF, 0x41, 0x7F, 0x80];

    with_harness_context(|mut context| {
        let _ = context.destack_process_env_delete_bytes(context.bytes_slice_value(&name)?);
        context.destack_process_env_set_bytes(
            context.bytes_slice_value(&name)?,
            context.bytes_slice_value(&value)?,
        )?;
        let value_out = context.destack_process_env_get_bytes(context.bytes_slice_value(&name)?)?;
        let value_out = context.bytes_from_array_value(value_out)?;
        assert_eq!(value_out, value);

        context.destack_process_env_delete_bytes(context.bytes_slice_value(&name)?)?;
        // deleted variables should report missing-variable errors
        assert_platform_error_code(
            context.destack_process_env_get_bytes(context.bytes_slice_value(&name)?),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Read the current working directory through process bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_process_cwd_roundtrip() {
    with_harness_context(|mut context| {
        let cwd = context.destack_process_cwd()?;
        let cwd = context.path_string_from_value(cwd)?;
        assert!(!cwd.is_empty());
        Ok(())
    });
}

/// Read pid and identity values through process bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_process_identity_reads() {
    with_harness_context(|mut context| {
        let pid = context.destack_process_pid()?;
        assert!(pid.0 > 0);

        // uid should be available or explicitly unsupported
        match context.destack_process_uid() {
            Ok(_uid) => {}
            Err(error) => {
                assert_platform_error_code::<UserId>(Err(error), PlatformErrorCode::NotSupported)?;
            }
        }

        // gid should be available or explicitly unsupported
        match context.destack_process_gid() {
            Ok(_gid) => {}
            Err(error) => {
                assert_platform_error_code::<GroupId>(Err(error), PlatformErrorCode::NotSupported)?;
            }
        }

        Ok(())
    });
}
