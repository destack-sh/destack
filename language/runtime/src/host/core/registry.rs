use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use super::{HostBridge, HostPlatform};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// Shared host-bridge runtime id generator.
static HOST_BRIDGE_ID_NEXT: AtomicU64 = AtomicU64::new(1);

/// Shared process-global host-bridge registry state.
static HOST_BRIDGE_REGISTRY: OnceLock<RwLock<HostBridgeRegistryState>> = OnceLock::new();

/// Registration guard for one host bridge.
#[derive(Debug)]
pub(crate) struct HostBridgeRegistration {
    /// Stable runtime id for this bridge.
    runtime_id: u64,
}

/// Shared registry state for active host bridges.
#[derive(Debug, Default)]
struct HostBridgeRegistryState {
    /// Bridge entries keyed by runtime id.
    bridges: FxHashMap<u64, HostBridgeRegistryEntry>,
}

/// Shared registry entry metadata for one host bridge.
#[derive(Debug)]
struct HostBridgeRegistryEntry {
    /// Platform tag for this bridge.
    platform: HostPlatform,
    /// Weak reference to one runtime-owned bridge.
    bridge: Weak<HostBridge>,
}

impl Drop for HostBridgeRegistration {
    fn drop(&mut self) {
        unregister_host_bridge(self.runtime_id);
    }
}

impl HostBridgeRegistration {
    /// Return the stable runtime id for this registration.
    pub(crate) fn runtime_id(&self) -> u64 {
        self.runtime_id
    }
}

/// Register one host bridge with one runtime id.
pub(crate) fn register_host_bridge(
    platform: HostPlatform,
    bridge: &Arc<HostBridge>,
) -> HostBridgeRegistration {
    let runtime_id = HOST_BRIDGE_ID_NEXT.fetch_add(1, Ordering::Relaxed);
    let mut state = host_bridge_registry().write();
    let entry = HostBridgeRegistryEntry {
        platform,
        bridge: Arc::downgrade(bridge),
    };
    state.bridges.insert(runtime_id, entry);

    HostBridgeRegistration { runtime_id }
}

/// Resolve one host bridge by runtime id and platform tag.
pub(crate) fn host_bridge_for_runtime(
    runtime_id: u64,
    platform: HostPlatform,
) -> RuntimeResult<Arc<HostBridge>> {
    let mut state = host_bridge_registry().write();
    let Some(entry) = state.bridges.get(&runtime_id) else {
        return Err(missing_host_bridge(runtime_id, platform));
    };

    if entry.platform != platform {
        return Err(missing_host_bridge(runtime_id, platform));
    }

    let Some(bridge) = entry.bridge.upgrade() else {
        state.bridges.remove(&runtime_id);
        return Err(missing_host_bridge(runtime_id, platform));
    };

    Ok(bridge)
}

/// Return the shared host bridge registry lock.
fn host_bridge_registry() -> &'static RwLock<HostBridgeRegistryState> {
    HOST_BRIDGE_REGISTRY.get_or_init(|| RwLock::new(HostBridgeRegistryState::default()))
}

/// Remove one registration from the shared host bridge registry.
fn unregister_host_bridge(runtime_id: u64) {
    let mut state = host_bridge_registry().write();
    let platform = state
        .bridges
        .remove(&runtime_id)
        .map(|entry| entry.platform);
    drop(state);

    cleanup_host_runtime_state(runtime_id, platform);
}

/// Cleanup host runtime state after one host-bridge unregistration.
fn cleanup_host_runtime_state(runtime_id: u64, platform: Option<HostPlatform>) {
    #[cfg(any(test, target_os = "android"))]
    {
        // remove android callback payloads eagerly when the bridge is dropped
        if platform == Some(HostPlatform::Android) {
            crate::host::android::unregister_android_bindings(runtime_id);
        }
    }

    #[cfg(not(any(test, target_os = "android")))]
    {
        let _ = (runtime_id, platform);
    }
}

/// Build one runtime error for missing host bridges.
fn missing_host_bridge(runtime_id: u64, platform: HostPlatform) -> Box<RuntimeError> {
    let platform = platform.canonical_tag();
    RuntimeError::from(PlatformError::not_supported(format!(
        "runtime.host.bridge.{platform}.{runtime_id}"
    )))
    .boxed()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{host_bridge_for_runtime, register_host_bridge};
    use crate::host::HostPlatform;
    use crate::host::core::{HostBridge, HostState};

    #[test]
    fn test_register_host_bridge_resolves_by_runtime_id() {
        let state = Arc::new(HostState::new());
        let bridge = Arc::new(HostBridge::new(state));
        let registration = register_host_bridge(HostPlatform::Android, &bridge);
        let runtime_id = registration.runtime_id();

        let resolved_bridge = host_bridge_for_runtime(runtime_id, HostPlatform::Android).unwrap();

        assert!(Arc::ptr_eq(&bridge, &resolved_bridge));
    }

    #[test]
    fn test_drop_registration_unregisters_runtime_id() {
        let state = Arc::new(HostState::new());
        let bridge = Arc::new(HostBridge::new(state));
        let registration = register_host_bridge(HostPlatform::MacOS, &bridge);
        let runtime_id = registration.runtime_id();

        drop(registration);

        let resolved_bridge = host_bridge_for_runtime(runtime_id, HostPlatform::MacOS);
        assert!(resolved_bridge.is_err());
    }

    #[test]
    fn test_host_bridge_for_runtime_rejects_platform_mismatch() {
        let state = Arc::new(HostState::new());
        let bridge = Arc::new(HostBridge::new(state));
        let registration = register_host_bridge(HostPlatform::Windows, &bridge);
        let runtime_id = registration.runtime_id();

        let resolved_bridge = host_bridge_for_runtime(runtime_id, HostPlatform::Android);
        assert!(resolved_bridge.is_err());
    }
}
