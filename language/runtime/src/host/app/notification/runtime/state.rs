#[cfg(test)]
use std::cell::Cell;
use std::sync::OnceLock;
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use rustc_hash::FxHashMap;

#[cfg(test)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::core::HostRuntimeId;
#[cfg(test)]
use crate::platform::PlatformError;
#[cfg(test)]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};

/// Shared runtime registry for desktop-style notification state.
static DESKTOP_NOTIFICATION_REGISTRY: OnceLock<Mutex<DesktopNotificationRegistry>> =
    OnceLock::new();

#[cfg(test)]
thread_local! {
    // thread-local native notification bypass used by runtime tests
    static DESKTOP_NOTIFICATION_TEST_MODE: Cell<bool> = const { Cell::new(false) };
}

/// RAII guard for one thread-local desktop notification test scope.
#[cfg(test)]
struct DesktopNotificationTestModeGuard {
    /// Previous test-mode state for this thread.
    previous: bool,
}

#[cfg(test)]
impl Drop for DesktopNotificationTestModeGuard {
    fn drop(&mut self) {
        DESKTOP_NOTIFICATION_TEST_MODE.with(|enabled| enabled.set(self.previous));
    }
}

/// Runtime-scoped desktop notification registry state.
#[derive(Debug, Default)]
pub(in crate::host::app::notification) struct DesktopNotificationRegistry {
    /// Notification state keyed by runtime id.
    pub(in crate::host::app::notification) runtimes:
        FxHashMap<HostRuntimeId, DesktopNotificationRuntimeState>,
}

/// Runtime-scoped notification state for one host session.
#[derive(Debug)]
pub(in crate::host::app::notification) struct DesktopNotificationRuntimeState {
    /// Registered notification categories.
    pub(super) categories: Vec<NotificationCategoryValue>,
    /// Posted notifications currently owned by this runtime.
    pub(super) posted: FxHashMap<String, NotificationRequestValue>,
    /// Pending scheduled notifications currently owned by this runtime.
    pub(super) pending: FxHashMap<String, NotificationScheduledDescriptorValue>,
    /// Last observed permission state for this runtime.
    pub(super) permission_state: NotificationPermissionState,
    /// Monotonic identifier sequence for host notification ids.
    pub(super) next_identifier: u64,
    /// Monotonic sequence for emitted notification events.
    pub(super) next_sequence: u64,
}

impl DesktopNotificationRuntimeState {
    /// Create one empty runtime notification state.
    pub(in crate::host::app::notification) fn new(_platform: Platform) -> Self {
        Self {
            categories: Vec::new(),
            posted: FxHashMap::default(),
            pending: FxHashMap::default(),
            permission_state: NotificationPermissionState::Prompt,
            next_identifier: 1,
            next_sequence: 0,
        }
    }

    /// Return the next emitted notification event sequence.
    pub(in crate::host::app::notification) fn next_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);

        sequence
    }
}

/// Run one callback with native notification side effects disabled.
#[cfg(test)]
pub(crate) fn with_notification_test_mode<T>(callback: impl FnOnce() -> T) -> T {
    let _guard = enter_desktop_notification_test_mode();

    callback()
}

/// Enter one thread-local desktop notification test scope.
#[cfg(test)]
fn enter_desktop_notification_test_mode() -> DesktopNotificationTestModeGuard {
    let previous = DESKTOP_NOTIFICATION_TEST_MODE.with(|enabled| enabled.replace(true));

    DesktopNotificationTestModeGuard { previous }
}

/// Return whether this thread bypasses native desktop notification side effects.
#[cfg(test)]
pub(in crate::host::app::notification) fn desktop_notification_test_mode_enabled() -> bool {
    DESKTOP_NOTIFICATION_TEST_MODE.with(Cell::get)
}

/// Return the shared desktop notification registry.
pub(in crate::host::app::notification) fn notification_registry()
-> &'static Mutex<DesktopNotificationRegistry> {
    DESKTOP_NOTIFICATION_REGISTRY.get_or_init(|| Mutex::new(DesktopNotificationRegistry::default()))
}

/// Return the current wall-clock time in Unix nanoseconds.
#[cfg(test)]
pub(in crate::host::app::notification) fn wall_clock_now_ns() -> RuntimeResult<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::Generic),
                format!("notification runtime clock moved before the Unix epoch: {error}"),
            ))
            .boxed()
        })?;

    Ok(now.as_nanos().min(u128::from(u64::MAX)) as u64)
}
