use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use super::{HostPlatform, HostState};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// Shared host runtime id generator.
static HOST_STATE_ID_NEXT: AtomicU64 = AtomicU64::new(1);

/// Shared process-global host state registry.
static HOST_STATE_REGISTRY: OnceLock<RwLock<HostStateRegistryState>> = OnceLock::new();

/// Registration guard for one host state.
#[derive(Debug)]
pub(crate) struct HostStateRegistration {
    /// Stable runtime id for this state.
    runtime_id: u64,
}

/// Shared registry state for active host states.
#[derive(Debug, Default)]
struct HostStateRegistryState {
    /// State entries keyed by runtime id.
    states: FxHashMap<u64, HostStateRegistryEntry>,
}

/// Shared registry entry metadata for one host state.
#[derive(Debug)]
struct HostStateRegistryEntry {
    /// Platform tag for this state.
    platform: HostPlatform,
    /// Weak reference to one runtime-owned state.
    state: Weak<HostState>,
}

impl Drop for HostStateRegistration {
    fn drop(&mut self) {
        unregister_host_state(self.runtime_id);
    }
}

impl HostStateRegistration {
    /// Return the stable runtime id for this registration.
    pub(crate) fn runtime_id(&self) -> u64 {
        self.runtime_id
    }
}

/// Register one host state with one runtime id.
pub(crate) fn register_host_state(
    platform: HostPlatform,
    host_state: &Arc<HostState>,
) -> HostStateRegistration {
    let runtime_id = HOST_STATE_ID_NEXT.fetch_add(1, Ordering::Relaxed);
    let mut state = host_state_registry().write();
    let entry = HostStateRegistryEntry {
        platform,
        state: Arc::downgrade(host_state),
    };
    state.states.insert(runtime_id, entry);

    HostStateRegistration { runtime_id }
}

/// Resolve one host state by runtime id and platform tag.
pub(crate) fn host_state_for_runtime(
    runtime_id: u64,
    platform: HostPlatform,
) -> RuntimeResult<Arc<HostState>> {
    let mut state = host_state_registry().write();
    let Some(entry) = state.states.get(&runtime_id) else {
        return Err(missing_host_state(runtime_id, platform));
    };

    if entry.platform != platform {
        return Err(missing_host_state(runtime_id, platform));
    }

    let Some(host_state) = entry.state.upgrade() else {
        state.states.remove(&runtime_id);
        return Err(missing_host_state(runtime_id, platform));
    };

    Ok(host_state)
}

/// Return the shared host state registry lock.
fn host_state_registry() -> &'static RwLock<HostStateRegistryState> {
    HOST_STATE_REGISTRY.get_or_init(|| RwLock::new(HostStateRegistryState::default()))
}

/// Remove one registration from the shared host state registry.
fn unregister_host_state(runtime_id: u64) {
    let mut state = host_state_registry().write();
    let platform = state.states.remove(&runtime_id).map(|entry| entry.platform);
    drop(state);

    cleanup_host_runtime_state(runtime_id, platform);
}

/// Cleanup host runtime state after one host-bridge unregistration.
fn cleanup_host_runtime_state(runtime_id: u64, platform: Option<HostPlatform>) {
    #[cfg(any(test, target_os = "android"))]
    {
        // remove android callback payloads eagerly when the state is dropped
        if platform == Some(HostPlatform::Android) {
            crate::host::android::unregister_android_bindings(runtime_id);
        }
    }

    #[cfg(not(any(test, target_os = "android")))]
    {
        let _ = (runtime_id, platform);
    }
}

/// Build one runtime error for missing host state.
fn missing_host_state(runtime_id: u64, platform: HostPlatform) -> Box<RuntimeError> {
    let platform = platform.canonical_tag();
    RuntimeError::from(PlatformError::not_supported(format!(
        "runtime.host.state.{platform}.{runtime_id}"
    )))
    .boxed()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{host_state_for_runtime, register_host_state};
    use crate::host::HostPlatform;
    use crate::host::core::HostState;

    #[test]
    fn test_register_host_state_resolves_by_runtime_id() {
        let host_state = Arc::new(HostState::new());
        let registration = register_host_state(HostPlatform::Android, &host_state);
        let runtime_id = registration.runtime_id();

        let resolved_state = host_state_for_runtime(runtime_id, HostPlatform::Android).unwrap();

        assert!(Arc::ptr_eq(&host_state, &resolved_state));
    }

    #[test]
    fn test_drop_registration_unregisters_runtime_id() {
        let host_state = Arc::new(HostState::new());
        let registration = register_host_state(HostPlatform::MacOS, &host_state);
        let runtime_id = registration.runtime_id();

        drop(registration);

        let resolved_state = host_state_for_runtime(runtime_id, HostPlatform::MacOS);
        assert!(resolved_state.is_err());
    }

    #[test]
    fn test_host_state_for_runtime_rejects_platform_mismatch() {
        let host_state = Arc::new(HostState::new());
        let registration = register_host_state(HostPlatform::Windows, &host_state);
        let runtime_id = registration.runtime_id();

        let resolved_state = host_state_for_runtime(runtime_id, HostPlatform::Android);
        assert!(resolved_state.is_err());
    }
}
