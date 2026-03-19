use crate::platform::os::tests::{decode_mount_entries_value, with_harness_context};

/// Verify mount table enumeration returns usable normalized entries.
#[test]
fn test_mount_list_returns_non_empty_targets() {
    with_harness_context(|mut context| {
        let entries = context.destack_os_mount_list()?;
        let entries = decode_mount_entries_value(&mut context, entries)?;

        assert!(!entries.is_empty());
        assert!(entries.iter().all(|entry| !entry.target_is_empty));
        assert!(entries.iter().all(|entry| {
            entry
                .source
                .as_ref()
                .is_none_or(|value| !value.trim().is_empty())
        }));
        assert!(entries.iter().all(|entry| {
            entry
                .file_system
                .as_ref()
                .is_none_or(|value| !value.trim().is_empty())
        }));

        Ok(())
    });
}
