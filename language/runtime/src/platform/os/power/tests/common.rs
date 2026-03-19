use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(any(target_os = "linux", windows))]
use crate::diagnostic::RuntimeResult;

#[cfg(target_os = "linux")]
use super::super::unix::set_test_suspend_hook;
#[cfg(target_os = "macos")]
use super::super::unix::set_test_suspend_hook;
#[cfg(windows)]
use super::super::windows::set_test_suspend_hook;

/// Shared suspend call counter used by supported-host suspend tests.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) static TEST_SUSPEND_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Shared mutex that serializes suspend-hook tests.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) static TEST_SUSPEND_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Install one suspend hook for the active backend.
#[cfg(any(target_os = "linux", windows))]
pub(super) fn install_test_suspend_hook(hook: Option<fn() -> RuntimeResult<()>>) {
    set_test_suspend_hook(hook);
}

/// Install one suspend hook for the macOS backend.
#[cfg(target_os = "macos")]
pub(super) fn install_test_suspend_hook(hook: Option<fn() -> i32>) {
    set_test_suspend_hook(hook);
}

/// Reset the installed suspend hook after one test case.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) struct SuspendHookGuard;

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
impl Drop for SuspendHookGuard {
    /// Clear the active suspend hook for the current backend.
    fn drop(&mut self) {
        install_test_suspend_hook(None);
    }
}

/// Report one successful linux suspend request without sleeping the machine.
#[cfg(target_os = "linux")]
pub(super) fn test_suspend_success_hook() -> RuntimeResult<()> {
    TEST_SUSPEND_CALL_COUNT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

/// Report one successful macOS suspend request without sleeping the machine.
#[cfg(target_os = "macos")]
pub(super) fn test_suspend_success_hook() -> i32 {
    TEST_SUSPEND_CALL_COUNT.fetch_add(1, Ordering::Relaxed);

    0
}

/// Report one successful windows suspend request without sleeping the machine.
#[cfg(windows)]
pub(super) fn test_suspend_success_hook() -> RuntimeResult<()> {
    TEST_SUSPEND_CALL_COUNT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}
