use std::sync::OnceLock;

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use super::bindings::AndroidHostBindings;
use crate::host::Platform;
use crate::host::abi::HostStatus;
use crate::host::core::HostRuntimeRegistry;
use crate::runtime::world::RuntimeId;

/// Shared Android bindings registry state.
#[derive(Debug, Default)]
struct AndroidBindingsRegistryState {
    /// Runtime-scoped callback bindings keyed by runtime id.
    bindings_by_runtime_id: FxHashMap<u64, AndroidHostBindings>,
}

/// Shared process-global Android bindings registry lock.
///
/// Native Android bridge callbacks are process-global symbols, so runtime-scoped
/// callback tables must live in one process-global registry keyed by runtime id.
static ANDROID_BINDINGS_REGISTRY: OnceLock<RwLock<AndroidBindingsRegistryState>> = OnceLock::new();

/// Return the shared Android bindings registry.
fn android_bindings_registry() -> &'static RwLock<AndroidBindingsRegistryState> {
    ANDROID_BINDINGS_REGISTRY.get_or_init(|| RwLock::new(AndroidBindingsRegistryState::default()))
}

/// Return whether one runtime id currently resolves to one Android host queue.
fn has_android_host_bridge(runtime_id: u64) -> bool {
    HostRuntimeRegistry::queue_for_runtime(RuntimeId(runtime_id), Platform::Android).is_ok()
}

/// Register one runtime-scoped Android bindings payload.
pub(crate) fn register_android_bindings(runtime_id: u64, bindings: AndroidHostBindings) -> u32 {
    // reject unknown runtime ids up front
    if !has_android_host_bridge(runtime_id) {
        return HostStatus::NotFound.code();
    }

    // insert one runtime-scoped bindings payload exactly once
    let mut registry = android_bindings_registry().write();
    if registry.bindings_by_runtime_id.contains_key(&runtime_id) {
        return HostStatus::Failed.code();
    }
    registry.bindings_by_runtime_id.insert(runtime_id, bindings);

    HostStatus::Ok.code()
}

/// Remove one runtime-scoped Android bindings payload.
#[cfg(any(test, target_os = "android"))]
pub(crate) fn unregister_android_bindings(runtime_id: u64) {
    // remove one runtime-scoped bindings payload when present
    let mut registry = android_bindings_registry().write();
    registry.bindings_by_runtime_id.remove(&runtime_id);
}

/// Resolve one runtime-scoped Android bindings snapshot.
pub(crate) fn resolve_android_bindings(runtime_id: u64) -> Result<AndroidHostBindings, u32> {
    // drop stale entries when the runtime state is already gone
    if !has_android_host_bridge(runtime_id) {
        let mut registry = android_bindings_registry().write();
        registry.bindings_by_runtime_id.remove(&runtime_id);
        return Err(HostStatus::NotFound.code());
    }

    // return one stable snapshot or report missing host registration
    let registry = android_bindings_registry().read();
    registry
        .bindings_by_runtime_id
        .get(&runtime_id)
        .copied()
        .ok_or(HostStatus::NotSupported.code())
}

#[cfg(test)]
mod tests {
    use super::ANDROID_BINDINGS_REGISTRY;
    use crate::host::abi::HostStatus;
    use crate::host::android::bridge::bindings::AndroidHostBindings;
    use crate::host::android::tests::{
        callback_test_lock, register_android_bindings, register_android_runtime,
    };

    /// Return whether the shared Android bindings registry still contains one runtime id.
    fn registry_contains_runtime_id(runtime_id: u64) -> bool {
        ANDROID_BINDINGS_REGISTRY.get().is_some_and(|registry| {
            registry
                .read()
                .bindings_by_runtime_id
                .contains_key(&runtime_id)
        })
    }

    /// Drop one runtime bridge and eagerly unregister its Android bindings.
    #[test]
    fn test_drop_runtime_bridge_unregisters_android_bindings_eagerly() {
        let _lock = callback_test_lock().lock().unwrap();
        let (_bridge, registration, runtime_id) = register_android_runtime();

        // register one android bindings payload for this runtime id
        let status = register_android_bindings(runtime_id, AndroidHostBindings::default());
        assert_eq!(status, HostStatus::Ok.code());
        assert!(registry_contains_runtime_id(runtime_id));

        // drop runtime registration and assert eager registry cleanup
        drop(registration);
        assert!(!registry_contains_runtime_id(runtime_id));
    }
}
