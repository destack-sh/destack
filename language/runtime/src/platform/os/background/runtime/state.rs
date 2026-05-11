#[cfg(test)]
use std::cell::Cell;
use std::sync::Arc;

use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::host::HostSessionId;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy};

use super::launch::desktop_background_registry_launch_marker_state;

/// Environment marker carrying one desktop background task identifier.
pub(crate) const DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV: &str =
    "DESTACK_BACKGROUND_TASK_IDENTIFIER";

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
pub(crate) struct DesktopBackgroundRegistry {
    /// Launch marker captured from the current process environment.
    pub(crate) launch_marker: Option<DesktopBackgroundLaunchMarker>,
    /// Launch marker initialization failure captured from the current process environment.
    pub(crate) launch_marker_error: Option<String>,
    /// Runtime that claimed the current launch marker.
    pub(crate) launch_session_id: Option<HostSessionId>,
    /// Runtime-scoped execution state.
    pub(crate) runtimes: FxHashMap<HostSessionId, DesktopBackgroundRuntimeState>,
}

impl Default for DesktopBackgroundRegistry {
    fn default() -> Self {
        let (launch_marker, launch_marker_error) =
            desktop_background_registry_launch_marker_state();

        Self {
            launch_marker,
            launch_marker_error,
            launch_session_id: None,
            runtimes: FxHashMap::default(),
        }
    }
}

/// Process launch marker for one desktop background execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopBackgroundLaunchMarker {
    /// Stable task identifier.
    pub(crate) identifier: String,
    /// Stable execution identifier.
    pub(crate) execution_id: String,
    /// UTC deadline in nanoseconds when provided.
    pub(crate) deadline_unix_ns: u64,
}

/// Runtime-scoped desktop background state.
#[derive(Debug, Default)]
pub(crate) struct DesktopBackgroundRuntimeState {
    /// Monotonic sequence for emitted background events.
    pub(crate) next_sequence: u64,
    /// Active executions keyed by execution id.
    pub(crate) executions: FxHashMap<String, DesktopBackgroundExecutionState>,
}

/// Runtime-scoped background execution lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopBackgroundExecutionState {
    /// Stable task identifier.
    pub(crate) identifier: String,
    /// Stable execution identifier.
    pub(crate) execution_id: String,
    /// UTC deadline in nanoseconds when provided.
    pub(crate) deadline_unix_ns: u64,
    /// Whether the execution already completed.
    pub(crate) is_completed: bool,
    /// Whether the runtime already emitted the expiration event.
    pub(crate) is_expired: bool,
}

/// Process-global desktop background runtime service.
pub(crate) struct DesktopBackgroundRuntimeService {
    /// Shared desktop background state.
    pub(crate) registry: Mutex<DesktopBackgroundRegistry>,
}

impl DesktopBackgroundRuntimeService {
    /// Create one empty desktop background runtime service.
    fn new() -> Self {
        Self {
            registry: Mutex::new(DesktopBackgroundRegistry::default()),
        }
    }
}

impl Service for DesktopBackgroundRuntimeService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
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
pub(crate) fn desktop_background_test_mode_enabled() -> bool {
    #[cfg(test)]
    {
        DESKTOP_BACKGROUND_TEST_MODE.with(Cell::get)
    }

    #[cfg(not(test))]
    {
        false
    }
}

/// Return the shared desktop background runtime service.
pub(crate) fn desktop_background_runtime_service() -> Arc<DesktopBackgroundRuntimeService> {
    match DesktopBackgroundRuntimeService::global(|| Ok(DesktopBackgroundRuntimeService::new())) {
        Ok(service) => service,
        Err(error) => {
            panic!("desktop background runtime service should be infallible: {error}");
        }
    }
}
