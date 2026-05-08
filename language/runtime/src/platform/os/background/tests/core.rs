use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex as StdMutex, OnceLock};

use destack_workspace::{AppBackgroundMode, RuntimeOptions};
use parking_lot::Mutex;

use crate::platform::os::background::with_background_test_mode;

/// Shared desktop background test state root override.
static DESKTOP_BACKGROUND_TEST_STATE_DIRECTORY: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
/// Shared mutex that serializes background test state overrides.
static DESKTOP_BACKGROUND_TEST_MUTEX: OnceLock<StdMutex<()>> = OnceLock::new();
/// Monotonic nonce for desktop background test directories.
static DESKTOP_BACKGROUND_TEST_NONCE: AtomicU64 = AtomicU64::new(1);

/// Run one background callback with deterministic scheduler state.
pub(super) fn with_background_test_environment<T>(label: &str, callback: impl FnOnce() -> T) -> T {
    let _guard = DESKTOP_BACKGROUND_TEST_MUTEX
        .get_or_init(|| StdMutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let _guard = install_desktop_background_test_state_directory(label);

    with_background_test_mode(callback)
}

/// Enable one background declaration and deterministic state root for tests.
pub(super) fn enable_background_declaration(options: &mut RuntimeOptions) {
    options
        .app
        .background
        .modes
        .push(AppBackgroundMode::Processing);
    options.os.state_directory = active_desktop_background_test_state_directory();
}

/// Return the active desktop background test state directory override.
fn active_desktop_background_test_state_directory() -> Option<PathBuf> {
    let slot = DESKTOP_BACKGROUND_TEST_STATE_DIRECTORY.get_or_init(|| Mutex::new(None));

    slot.lock().clone()
}

/// Install one deterministic desktop background state root for one test case.
fn install_desktop_background_test_state_directory(
    label: &str,
) -> DesktopBackgroundStateDirectoryGuard {
    let nonce = DESKTOP_BACKGROUND_TEST_NONCE.fetch_add(1, Ordering::Relaxed);
    let process_id = std::process::id();
    let base_directory = std::env::temp_dir().join(format!(
        "destack-os-background-{label}-{process_id}-{nonce}"
    ));

    std::fs::create_dir_all(&base_directory)
        .expect("desktop background state directory should create");

    let slot = DESKTOP_BACKGROUND_TEST_STATE_DIRECTORY.get_or_init(|| Mutex::new(None));
    *slot.lock() = Some(base_directory.clone());

    DesktopBackgroundStateDirectoryGuard { base_directory }
}

/// One scoped desktop background state-directory installation.
struct DesktopBackgroundStateDirectoryGuard {
    /// The temporary state root for this test case.
    base_directory: PathBuf,
}

impl Drop for DesktopBackgroundStateDirectoryGuard {
    /// Clear the active desktop background state root after one test.
    fn drop(&mut self) {
        let slot = DESKTOP_BACKGROUND_TEST_STATE_DIRECTORY.get_or_init(|| Mutex::new(None));
        *slot.lock() = None;
        let _ = std::fs::remove_dir_all(&self.base_directory);
    }
}
