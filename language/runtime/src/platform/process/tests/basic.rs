use super::{assert_platform_error_code, unique_env_name, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{GroupId, UserId};

/// Read process arguments through native and vm harnesses.
#[test]
fn test_process_args_roundtrip() {
    with_harness_context(|mut context| {
        let args = context.args()?;
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

/// Set, read, and delete one utf8 environment variable.
#[cfg(any(unix, windows))]
#[test]
fn test_process_env_roundtrip() {
    let name = unique_env_name("UTF8");
    let value = "destack-process-env";

    with_harness_context(|mut context| {
        let _ = context.env_delete(&name);
        context.env_set(&name, value)?;
        assert_eq!(context.env_get(&name)?, value);

        context.env_delete(&name)?;
        // deleted variables should report missing-variable errors
        assert_platform_error_code(
            context.env_get(&name),
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
        let _ = context.env_delete_bytes(&name);
        context.env_set_bytes(&name, &value)?;
        assert_eq!(context.env_get_bytes(&name)?, value);

        context.env_delete_bytes(&name)?;
        // deleted variables should report missing-variable errors
        assert_platform_error_code(
            context.env_get_bytes(&name),
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
        let cwd = context.cwd()?;
        assert!(!cwd.is_empty());
        Ok(())
    });
}

/// Read pid and identity values through process bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_process_identity_reads() {
    with_harness_context(|mut context| {
        let pid = context.pid()?;
        assert!(pid.0 > 0);

        // uid should be available or explicitly unsupported
        match context.uid() {
            Ok(_uid) => {}
            Err(error) => {
                assert_platform_error_code::<UserId>(Err(error), PlatformErrorCode::NotSupported)?;
            }
        }

        // gid should be available or explicitly unsupported
        match context.gid() {
            Ok(_gid) => {}
            Err(error) => {
                assert_platform_error_code::<GroupId>(Err(error), PlatformErrorCode::NotSupported)?;
            }
        }

        Ok(())
    });
}
