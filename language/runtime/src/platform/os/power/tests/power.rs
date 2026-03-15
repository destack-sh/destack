#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeResult;
#[cfg(windows)]
use crate::diagnostic::RuntimeResult;
use crate::platform::os::PowerState;
use crate::platform::os::tests::with_harness_context;
use crate::tests::platform::error_code_from_result;

#[cfg(target_os = "linux")]
use crate::platform::os::power::unix::set_test_suspend_hook;
#[cfg(target_os = "macos")]
use crate::platform::os::power::unix::set_test_suspend_hook;
#[cfg(windows)]
use crate::platform::os::power::windows::set_test_suspend_hook;

/// Shared suspend call counter used by supported-host suspend tests.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
static TEST_SUSPEND_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Shared mutex that serializes suspend-hook tests.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
static TEST_SUSPEND_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Reset the installed suspend hook after one test case.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
struct SuspendHookGuard;

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
impl Drop for SuspendHookGuard {
    /// Clear the active suspend hook for the current backend.
    fn drop(&mut self) {
        set_test_suspend_hook(None);
    }
}

/// Report one successful linux suspend request without sleeping the machine.
#[cfg(target_os = "linux")]
fn test_suspend_success_hook() -> RuntimeResult<()> {
    TEST_SUSPEND_CALL_COUNT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

/// Report one successful macOS suspend request without sleeping the machine.
#[cfg(target_os = "macos")]
fn test_suspend_success_hook() -> i32 {
    TEST_SUSPEND_CALL_COUNT.fetch_add(1, Ordering::Relaxed);

    0
}

/// Report one successful windows suspend request without sleeping the machine.
#[cfg(windows)]
fn test_suspend_success_hook() -> RuntimeResult<()> {
    TEST_SUSPEND_CALL_COUNT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

/// Verify power-state reads follow the supported host contract.
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    windows
))]
#[test]
fn test_power_state_returns_normalized_variant() {
    with_harness_context(|mut context| {
        // supported host lanes should return one normalized state
        let state = context.destack_os_power_state()?;
        match state {
            PowerState::AC | PowerState::Battery | PowerState::Unknown => {}
        }

        Ok(())
    });
}

/// Verify suspend succeeds on supported desktop hosts without sleeping the test machine.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_suspend_requests_supported_host_transition() {
    let _guard = TEST_SUSPEND_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    TEST_SUSPEND_CALL_COUNT.store(0, Ordering::Relaxed);
    set_test_suspend_hook(Some(test_suspend_success_hook));
    let _guard = SuspendHookGuard;

    with_harness_context(|mut context| {
        context.destack_os_suspend()?;

        Ok(())
    });

    assert_eq!(TEST_SUSPEND_CALL_COUNT.load(Ordering::Relaxed), 2);
}

/// Verify suspend tests fail closed when no test hook is installed.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_suspend_requires_test_hook_on_supported_hosts() {
    let _guard = TEST_SUSPEND_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    TEST_SUSPEND_CALL_COUNT.store(0, Ordering::Relaxed);
    set_test_suspend_hook(None);

    with_harness_context(|mut context| {
        let result = context.destack_os_suspend();

        // supported-host test builds must fail closed instead of touching the live os path
        let error_code = error_code_from_result(result)
            .expect("missing suspend hook should decode one platform error");

        assert_eq!(
            error_code,
            crate::platform::diagnostic::PlatformErrorCode::Generic
        );

        Ok(())
    });

    assert_eq!(TEST_SUSPEND_CALL_COUNT.load(Ordering::Relaxed), 0);
}
