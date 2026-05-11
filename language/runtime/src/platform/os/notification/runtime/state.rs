#[cfg(test)]
use std::cell::Cell;
use std::sync::Arc;
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use rustc_hash::FxHashMap;

#[cfg(test)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostSessionId, Platform};
#[cfg(test)]
use crate::platform::PlatformError;
#[cfg(test)]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::NotificationPermissionState;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationRequestValue, NotificationScheduledDescriptorValue,
};
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy};

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
pub(crate) struct DesktopNotificationRegistry {
    /// Notification state keyed by runtime id.
    pub(crate) runtimes: FxHashMap<HostSessionId, DesktopNotificationRuntimeState>,
}

/// Runtime-scoped notification state for one host session.
#[derive(Debug)]
pub(crate) struct DesktopNotificationRuntimeState {
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

/// Process-global desktop notification runtime service.
pub(crate) struct DesktopNotificationRuntimeService {
    /// Shared runtime registry for desktop-style notification state.
    pub(crate) registry: Mutex<DesktopNotificationRegistry>,
}

impl DesktopNotificationRuntimeState {
    /// Create one empty runtime notification state.
    pub(crate) fn new(_platform: Platform) -> Self {
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
    pub(crate) fn next_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);

        sequence
    }
}

impl DesktopNotificationRuntimeService {
    /// Create one empty desktop notification runtime service.
    fn new() -> Self {
        Self {
            registry: Mutex::new(DesktopNotificationRegistry::default()),
        }
    }
}

impl Service for DesktopNotificationRuntimeService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
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
pub(crate) fn desktop_notification_test_mode_enabled() -> bool {
    DESKTOP_NOTIFICATION_TEST_MODE.with(Cell::get)
}

/// Return the shared desktop notification runtime service.
pub(crate) fn notification_runtime_service() -> Arc<DesktopNotificationRuntimeService> {
    match DesktopNotificationRuntimeService::global(|| Ok(DesktopNotificationRuntimeService::new()))
    {
        Ok(service) => service,
        Err(error) => {
            panic!("desktop notification runtime service should be infallible: {error}");
        }
    }
}

/// Return one live notification request for one runtime notification identifier.
pub(crate) fn notification_request(
    host_session_id: HostSessionId,
    id: &str,
) -> Option<NotificationRequestValue> {
    let service = notification_runtime_service();
    let registry = service.registry.lock();
    let runtime_state = registry.runtimes.get(&host_session_id)?;

    // posted first
    if let Some(request) = runtime_state.posted.get(id) {
        return Some(request.clone());
    }

    // scheduled fallback
    runtime_state
        .pending
        .get(id)
        .map(|descriptor| descriptor.request.clone())
}

/// Return one live pending notification descriptor for one runtime notification identifier.
#[cfg(windows)]
pub(crate) fn pending_notification(
    host_session_id: HostSessionId,
    id: &str,
) -> Option<NotificationScheduledDescriptorValue> {
    let service = notification_runtime_service();
    let registry = service.registry.lock();
    let runtime_state = registry.runtimes.get(&host_session_id)?;

    runtime_state.pending.get(id).cloned()
}

/// Return every live pending notification descriptor for one runtime.
pub(crate) fn pending_notifications(
    host_session_id: HostSessionId,
) -> Vec<NotificationScheduledDescriptorValue> {
    let service = notification_runtime_service();
    let registry = service.registry.lock();
    let Some(runtime_state) = registry.runtimes.get(&host_session_id) else {
        return Vec::new();
    };

    runtime_state.pending.values().cloned().collect()
}

/// Upsert one live pending notification descriptor for one runtime.
pub(crate) fn upsert_pending_notification(
    host_session_id: HostSessionId,
    platform: Platform,
    descriptor: NotificationScheduledDescriptorValue,
) {
    let service = notification_runtime_service();
    let mut registry = service.registry.lock();
    let runtime_state = registry
        .runtimes
        .entry(host_session_id)
        .or_insert_with(|| DesktopNotificationRuntimeState::new(platform));

    runtime_state
        .pending
        .insert(descriptor.id.clone(), descriptor);
}

/// Remove one live notification request from posted and pending runtime state.
pub(crate) fn remove_notification_request(host_session_id: HostSessionId, id: &str) {
    let service = notification_runtime_service();
    let mut registry = service.registry.lock();

    if let Some(runtime_state) = registry.runtimes.get_mut(&host_session_id) {
        runtime_state.posted.remove(id);
        runtime_state.pending.remove(id);
    }
}

/// Return the current wall-clock time in Unix nanoseconds.
#[cfg(test)]
pub(crate) fn wall_clock_now_ns() -> RuntimeResult<u64> {
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
