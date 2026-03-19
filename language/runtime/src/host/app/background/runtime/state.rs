#[cfg(test)]
use std::cell::Cell;
use std::sync::OnceLock;

use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::host::core::HostRuntimeId;

use super::launch::desktop_background_registry_launch_marker_state;

/// Environment marker carrying one desktop background task identifier.
pub(in crate::host::app::background) const DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV: &str =
    "DESTACK_BACKGROUND_TASK_IDENTIFIER";

/// Shared desktop background state.
static DESKTOP_BACKGROUND_REGISTRY: OnceLock<Mutex<DesktopBackgroundRegistry>> = OnceLock::new();

#[cfg(test)]
thread_local! {
    // thread-local desktop background scheduler bypass used by runtime tests
    static DESKTOP_BACKGROUND_TEST_MODE: Cell<bool> = const { Cell::new(false) };
}

/// RAII guard for one thread-local desktop background test scope.
#[cfg(test)]
struct DesktopBackgroundTestModeGuard {
    /// Previous test-mode state for this thread.
    previous: bool,
}

#[cfg(test)]
impl Drop for DesktopBackgroundTestModeGuard {
    fn drop(&mut self) {
        DESKTOP_BACKGROUND_TEST_MODE.with(|enabled| enabled.set(self.previous));
    }
}

/// Process-wide desktop background registry.
#[derive(Debug)]
pub(in crate::host::app::background) struct DesktopBackgroundRegistry {
    /// Launch marker captured from the current process environment.
    pub(in crate::host::app::background) launch_marker: Option<DesktopBackgroundLaunchMarker>,
    /// Launch marker initialization failure captured from the current process environment.
    pub(in crate::host::app::background) launch_marker_error: Option<String>,
    /// Runtime that claimed the current launch marker.
    pub(in crate::host::app::background) launch_runtime_id: Option<HostRuntimeId>,
    /// Runtime-scoped execution state.
    pub(in crate::host::app::background) runtimes:
        FxHashMap<HostRuntimeId, DesktopBackgroundRuntimeState>,
}

impl Default for DesktopBackgroundRegistry {
    fn default() -> Self {
        let (launch_marker, launch_marker_error) =
            desktop_background_registry_launch_marker_state();

        Self {
            launch_marker,
            launch_marker_error,
            launch_runtime_id: None,
            runtimes: FxHashMap::default(),
        }
    }
}

/// Process launch marker for one desktop background execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::host::app::background) struct DesktopBackgroundLaunchMarker {
    /// Stable task identifier.
    pub(in crate::host::app::background) identifier: String,
    /// Stable execution identifier.
    pub(in crate::host::app::background) execution_id: String,
    /// UTC deadline in nanoseconds when provided.
    pub(in crate::host::app::background) deadline_unix_ns: u64,
}

/// Runtime-scoped desktop background state.
#[derive(Debug, Default)]
pub(in crate::host::app::background) struct DesktopBackgroundRuntimeState {
    /// Monotonic sequence for emitted background events.
    pub(in crate::host::app::background) next_sequence: u64,
    /// Active executions keyed by execution id.
    pub(in crate::host::app::background) executions:
        FxHashMap<String, DesktopBackgroundExecutionState>,
}

/// Runtime-scoped background execution lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::host::app::background) struct DesktopBackgroundExecutionState {
    /// Stable task identifier.
    pub(in crate::host::app::background) identifier: String,
    /// Stable execution identifier.
    pub(in crate::host::app::background) execution_id: String,
    /// UTC deadline in nanoseconds when provided.
    pub(in crate::host::app::background) deadline_unix_ns: u64,
    /// Whether the execution already completed.
    pub(in crate::host::app::background) is_completed: bool,
    /// Whether the runtime already emitted the expiration event.
    pub(in crate::host::app::background) is_expired: bool,
}

/// Run one callback with external background scheduler side effects disabled.
#[cfg(test)]
pub(crate) fn with_background_test_mode<T>(callback: impl FnOnce() -> T) -> T {
    let _guard = enter_desktop_background_test_mode();

    callback()
}

/// Enter one thread-local desktop background test scope.
#[cfg(test)]
fn enter_desktop_background_test_mode() -> DesktopBackgroundTestModeGuard {
    let previous = DESKTOP_BACKGROUND_TEST_MODE.with(|enabled| enabled.replace(true));

    DesktopBackgroundTestModeGuard { previous }
}

/// Return whether this thread bypasses external desktop scheduler side effects.
pub(in crate::host::app::background) fn desktop_background_test_mode_enabled() -> bool {
    #[cfg(test)]
    {
        DESKTOP_BACKGROUND_TEST_MODE.with(Cell::get)
    }

    #[cfg(not(test))]
    {
        false
    }
}

/// Return the shared desktop background registry.
pub(in crate::host::app::background) fn desktop_background_registry()
-> &'static Mutex<DesktopBackgroundRegistry> {
    DESKTOP_BACKGROUND_REGISTRY.get_or_init(|| Mutex::new(DesktopBackgroundRegistry::default()))
}
