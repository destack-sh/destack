use super::{allow_not_supported, unique_env_name, with_harness_context};

#[test]
fn test_process_args_roundtrip() {
    with_harness_context(|mut context| {
        let args = context.args()?;
        assert!(args.is_empty());
        Ok(())
    });
}

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
        let missing = context.env_get(&name);
        assert!(missing.is_err());

        Ok(())
    });
}

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
        let missing = context.env_get_bytes(&name);
        assert!(missing.is_err());

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_process_cwd_roundtrip() {
    with_harness_context(|mut context| {
        let cwd = context.cwd()?;
        assert!(!cwd.is_empty());
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_process_identity_reads() {
    with_harness_context(|mut context| {
        let pid = context.pid()?;
        assert!(pid.0 > 0);

        let uid = allow_not_supported(context.uid())?;
        if let Some(uid) = uid {
            assert!(uid.0 > 0);
        }

        let gid = allow_not_supported(context.gid())?;
        if let Some(gid) = gid {
            assert!(gid.0 > 0);
        }

        Ok(())
    });
}
