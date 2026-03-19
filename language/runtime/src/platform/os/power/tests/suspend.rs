use std::sync::Mutex;
use std::sync::atomic::Ordering;

use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::tests::with_harness_context;
use crate::tests::platform::error_code_from_result;

use super::common::{
    SuspendHookGuard, TEST_SUSPEND_CALL_COUNT, TEST_SUSPEND_MUTEX, install_test_suspend_hook,
    test_suspend_success_hook,
};

/// Verify suspend succeeds on supported desktop hosts without sleeping the test machine.
#[test]
fn test_suspend_requests_supported_host_transition() {
    let _guard = TEST_SUSPEND_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    TEST_SUSPEND_CALL_COUNT.store(0, Ordering::Relaxed);
    install_test_suspend_hook(Some(test_suspend_success_hook));
    let _guard = SuspendHookGuard;

    with_harness_context(|mut context| {
        context.destack_os_suspend()?;

        Ok(())
    });

    assert_eq!(TEST_SUSPEND_CALL_COUNT.load(Ordering::Relaxed), 2);
}

/// Verify suspend tests fail closed when no test hook is installed.
#[test]
fn test_suspend_requires_test_hook_on_supported_hosts() {
    let _guard = TEST_SUSPEND_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    TEST_SUSPEND_CALL_COUNT.store(0, Ordering::Relaxed);
    install_test_suspend_hook(None);

    with_harness_context(|mut context| {
        let result = context.destack_os_suspend();
        let error_code = error_code_from_result(result)
            .expect("missing suspend hook should decode one platform error");

        assert_eq!(error_code, PlatformErrorCode::Generic);

        Ok(())
    });

    assert_eq!(TEST_SUSPEND_CALL_COUNT.load(Ordering::Relaxed), 0);
}
