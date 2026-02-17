use super::{assert_platform_error_code, with_harness_context};

use crate::platform::diagnostic::PlatformErrorCode;

/// Create one thread-local key and roundtrip one value through it.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_local_roundtrip() {
    with_harness_context(|mut context| {
        let key = context.local_create()?;

        context.local_set(key, 0xfeed_face_dead_beef)?;
        let value = context.local_get(key)?;

        // the value should roundtrip exactly
        assert_eq!(value, 0xfeed_face_dead_beef);

        context.local_delete(key)?;

        Ok(())
    });
}

/// Reject reads from one deleted thread-local key.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_local_reject_deleted_key() {
    with_harness_context(|mut context| {
        let key = context.local_create()?;
        context.local_delete(key)?;

        let result = context.local_get(key);
        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}
