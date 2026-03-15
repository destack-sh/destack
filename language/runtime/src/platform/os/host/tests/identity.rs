use crate::platform::os::tests::{decode_host_identity_value, with_harness_context};

/// Verify host identity returns non-empty normalized fields across native and VM bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_host_identity_returns_non_empty_fields() {
    with_harness_context(|mut context| {
        // read one host identity payload and decode host strings
        let identity = context.destack_os_host_identity()?;
        let (hostname, kernel, release, architecture) =
            decode_host_identity_value(&mut context, identity)?;

        // windows should report one concrete normalized identity shape
        if cfg!(windows) {
            let release_parts = release.split('.').collect::<Vec<_>>();

            assert_eq!(kernel, "windows");
            assert_eq!(release_parts.len(), 3);
            assert!(release_parts.iter().all(|part| !part.is_empty()));
            assert!(
                release_parts
                    .iter()
                    .all(|part| part.chars().all(|character| character.is_ascii_digit()))
            );
        }

        // verify each exported identity field contains usable text
        assert!(!hostname.trim().is_empty());
        assert!(!kernel.trim().is_empty());
        assert!(!release.trim().is_empty());
        assert!(!architecture.trim().is_empty());

        Ok(())
    });
}
