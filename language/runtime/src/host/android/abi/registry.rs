use std::sync::OnceLock;

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use super::binding::AndroidHostBindings;
use crate::host::{HostSessionHandle, HostSessionId, HostSessionRegistry, HostStatus, Platform};

/// Shared Android bindings registry state.
#[derive(Debug, Default)]
struct AndroidBindingsRegistryState {
    /// Runtime-scoped callback bindings keyed by runtime id.
    bindings_by_session_handle: FxHashMap<HostSessionHandle, AndroidHostBindings>,
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
fn has_android_host_bridge(session_handle: HostSessionHandle) -> bool {
    let host_session_id = HostSessionId::from_handle(session_handle);

    HostSessionRegistry::queue_for_session(host_session_id, Platform::Android).is_ok()
}

/// Register one runtime-scoped Android bindings payload.
pub(crate) fn register_android_bindings(
    session_handle: HostSessionHandle,
    bindings: AndroidHostBindings,
) -> u32 {
    // reject unknown runtime ids up front
    if !has_android_host_bridge(session_handle) {
        return HostStatus::NotFound.code();
    }

    // insert one runtime-scoped bindings payload exactly once
    let mut registry = android_bindings_registry().write();
    if registry
        .bindings_by_session_handle
        .contains_key(&session_handle)
    {
        return HostStatus::Failed.code();
    }
    registry
        .bindings_by_session_handle
        .insert(session_handle, bindings);

    HostStatus::Ok.code()
}

/// Remove one runtime-scoped Android bindings payload.
pub(crate) fn unregister_android_bindings(session_handle: HostSessionHandle) {
    // remove one runtime-scoped bindings payload when present
    let mut registry = android_bindings_registry().write();
    registry.bindings_by_session_handle.remove(&session_handle);
}

/// Resolve one runtime-scoped Android bindings snapshot.
pub(crate) fn resolve_android_bindings(
    session_handle: HostSessionHandle,
) -> Result<AndroidHostBindings, u32> {
    // drop stale entries when the runtime state is already gone
    if !has_android_host_bridge(session_handle) {
        let mut registry = android_bindings_registry().write();
        registry.bindings_by_session_handle.remove(&session_handle);
        return Err(HostStatus::NotFound.code());
    }

    // return one stable snapshot or report missing host registration
    let registry = android_bindings_registry().read();
    registry
        .bindings_by_session_handle
        .get(&session_handle)
        .copied()
        .ok_or(HostStatus::NotSupported.code())
}

#[cfg(test)]
mod tests {
    use super::ANDROID_BINDINGS_REGISTRY;
    use crate::host::HostStatus;
    use crate::host::os::android::abi::binding::AndroidHostBindings;
    use crate::host::os::android::tests::{
        callback_test_lock, register_android_bindings, register_android_runtime,
    };

    /// Return whether the shared Android bindings registry still contains one runtime id.
    fn registry_contains_runtime_id(runtime_id: u64) -> bool {
        ANDROID_BINDINGS_REGISTRY.get().is_some_and(|registry| {
            registry
                .read()
                .bindings_by_session_handle
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
