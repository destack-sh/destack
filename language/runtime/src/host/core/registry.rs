use std::sync::{Arc, OnceLock, Weak};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use super::error::missing_host_state;
use super::observer::cleanup_runtime_ingress_observers;
use super::{HostState, Platform};
use crate::diagnostic::RuntimeResult;
use crate::runtime::world::RuntimeId;

/// Cleanup hook run when one runtime host-state registration is removed.
pub(crate) type HostCleanup = fn(runtime_id: u64);

/// Shared process-global host-state registry.
static HOST_RUNTIME_STATE_REGISTRY: OnceLock<RwLock<HostStateRegistryState>> = OnceLock::new();

/// Registration guard for one host runtime state.
#[derive(Debug)]
pub(crate) struct HostRegistrationGuard {
    /// Stable runtime id for this state.
    runtime_id: RuntimeId,
}

/// Shared registry state for active host runtime states.
#[derive(Debug, Default)]
struct HostStateRegistryState {
    /// State entries keyed by runtime id.
    states: FxHashMap<RuntimeId, HostStateRegistryEntry>,
}

/// Shared registry entry metadata for one host runtime state.
#[derive(Debug)]
struct HostStateRegistryEntry {
    /// Platform tag for this state.
    platform: Platform,
    /// Weak reference to one runtime-owned state.
    state: Weak<HostState>,
    /// Optional platform-specific cleanup hook for this runtime id.
    cleanup: Option<HostCleanup>,
}

impl HostRegistrationGuard {
    /// Return the stable runtime id for this registration.
    pub(crate) fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }
}

impl Drop for HostRegistrationGuard {
    fn drop(&mut self) {
        unregister_host_state(self.runtime_id);
    }
}

/// Register one host runtime state with one runtime id.
pub(crate) fn register_host_state(
    platform: Platform,
    runtime_id: RuntimeId,
    runtime_state: Weak<HostState>,
    cleanup: Option<HostCleanup>,
) -> HostRegistrationGuard {
    let mut state = host_runtime_state_registry().write();
    let entry = HostStateRegistryEntry {
        platform,
        state: runtime_state,
        cleanup,
    };
    state.states.insert(runtime_id, entry);

    HostRegistrationGuard { runtime_id }
}

/// Resolve one host runtime state by runtime id and platform tag.
pub(crate) fn host_state_for_runtime(
    runtime_id: RuntimeId,
    platform: Platform,
) -> RuntimeResult<Arc<HostState>> {
    let mut state = host_runtime_state_registry().write();
    let Some(entry) = state.states.get(&runtime_id) else {
        return Err(missing_host_state(runtime_id.0, platform));
    };

    if entry.platform != platform {
        return Err(missing_host_state(runtime_id.0, platform));
    }

    let Some(runtime_state) = entry.state.upgrade() else {
        state.states.remove(&runtime_id);
        return Err(missing_host_state(runtime_id.0, platform));
    };

    Ok(runtime_state)
}

/// Return the shared host runtime-state registry lock.
fn host_runtime_state_registry() -> &'static RwLock<HostStateRegistryState> {
    HOST_RUNTIME_STATE_REGISTRY.get_or_init(|| RwLock::new(HostStateRegistryState::default()))
}

/// Remove one registration from the shared host runtime-state registry.
fn unregister_host_state(runtime_id: RuntimeId) {
    let mut state = host_runtime_state_registry().write();
    let cleanup = state
        .states
        .remove(&runtime_id)
        .and_then(|entry| entry.cleanup);
    drop(state);

    cleanup_runtime_ingress_observers(runtime_id.0);

    if let Some(cleanup) = cleanup {
        cleanup(runtime_id.0);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{host_state_for_runtime, register_host_state};
    use crate::host::Platform;
    use crate::host::core::HostState;
    use crate::runtime::world::RuntimeId;

    /// Shared runtime id captured by one cleanup hook invocation in tests.
    static TEST_CLEANUP_RUNTIME_ID: AtomicU64 = AtomicU64::new(0);

    /// Record one cleanup hook invocation for tests.
    fn test_cleanup_hook(runtime_id: u64) {
        TEST_CLEANUP_RUNTIME_ID.store(runtime_id, Ordering::Relaxed);
    }

    #[test]
    fn test_register_host_state_resolves_by_runtime_id() {
        let runtime_state = HostState::new_for_test();
        let registration = register_host_state(
            Platform::Android,
            RuntimeId(1),
            Arc::downgrade(&runtime_state),
            None,
        );
        let runtime_id = registration.runtime_id;

        let resolved_state = host_state_for_runtime(runtime_id, Platform::Android).unwrap();

        assert!(Arc::ptr_eq(&runtime_state, &resolved_state));
    }

    #[test]
    fn test_drop_registration_unregisters_runtime_id() {
        let runtime_state = HostState::new_for_test();
        let registration = register_host_state(
            Platform::MacOS,
            RuntimeId(2),
            Arc::downgrade(&runtime_state),
            None,
        );
        let runtime_id = registration.runtime_id;

        drop(registration);

        let resolved_state = host_state_for_runtime(runtime_id, Platform::MacOS);
        assert!(resolved_state.is_err());
    }

    #[test]
    fn test_host_state_for_runtime_rejects_platform_mismatch() {
        let runtime_state = HostState::new_for_test();
        let registration = register_host_state(
            Platform::Windows,
            RuntimeId(3),
            Arc::downgrade(&runtime_state),
            None,
        );
        let runtime_id = registration.runtime_id;

        let resolved_state = host_state_for_runtime(runtime_id, Platform::Android);
        assert!(resolved_state.is_err());
    }

    #[test]
    fn test_drop_registration_runs_cleanup_hook() {
        TEST_CLEANUP_RUNTIME_ID.store(0, Ordering::Relaxed);

        let runtime_state = HostState::new_for_test();
        let registration = register_host_state(
            Platform::Android,
            RuntimeId(4),
            Arc::downgrade(&runtime_state),
            Some(test_cleanup_hook),
        );
        let runtime_id = registration.runtime_id;

        drop(registration);

        let cleaned_runtime_id = TEST_CLEANUP_RUNTIME_ID.load(Ordering::Relaxed);
        assert_eq!(cleaned_runtime_id, runtime_id.0);
    }
}
