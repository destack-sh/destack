use std::sync::OnceLock;

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::host::core::HostQueueRegistry;
use crate::host::ios::bridge::bindings::IosHostBindings;
use crate::host::{
    HOST_STATUS_FAILED, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK, Platform,
};
use crate::runtime::world::RuntimeId;

/// Shared iOS bindings registry state.
#[derive(Debug, Default)]
struct IosBindingsRegistryState {
    /// Runtime-scoped callback bindings keyed by runtime id.
    bindings_by_runtime_id: FxHashMap<u64, IosHostBindings>,
}

/// Shared process-global iOS bindings registry lock.
///
/// Native iOS bridge callbacks are process-global symbols, so runtime-scoped
/// callback tables must live in one process-global registry keyed by runtime id.
static IOS_BINDINGS_REGISTRY: OnceLock<RwLock<IosBindingsRegistryState>> = OnceLock::new();

/// Return the shared iOS bindings registry.
fn ios_bindings_registry() -> &'static RwLock<IosBindingsRegistryState> {
    IOS_BINDINGS_REGISTRY.get_or_init(|| RwLock::new(IosBindingsRegistryState::default()))
}

/// Return whether one runtime id currently resolves to one iOS host queue.
fn has_ios_host_bridge(runtime_id: u64) -> bool {
    HostQueueRegistry::shared()
        .write()
        .queue_for_runtime(RuntimeId(runtime_id), Platform::IOS)
        .is_ok()
}

/// Register one runtime-scoped iOS bindings payload.
pub(crate) fn register_ios_bindings(runtime_id: u64, bindings: IosHostBindings) -> u32 {
    if !has_ios_host_bridge(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    let mut registry = ios_bindings_registry().write();
    if registry.bindings_by_runtime_id.contains_key(&runtime_id) {
        return HOST_STATUS_FAILED;
    }
    registry.bindings_by_runtime_id.insert(runtime_id, bindings);

    HOST_STATUS_OK
}

/// Remove one runtime-scoped iOS bindings payload.
#[cfg(any(test, target_os = "ios"))]
pub(crate) fn unregister_ios_bindings(runtime_id: u64) {
    let mut registry = ios_bindings_registry().write();
    registry.bindings_by_runtime_id.remove(&runtime_id);
}

/// Resolve one runtime-scoped iOS bindings snapshot.
pub(crate) fn resolve_ios_bindings(runtime_id: u64) -> Result<IosHostBindings, u32> {
    if !has_ios_host_bridge(runtime_id) {
        let mut registry = ios_bindings_registry().write();
        registry.bindings_by_runtime_id.remove(&runtime_id);
        return Err(HOST_STATUS_NOT_FOUND);
    }

    let registry = ios_bindings_registry().read();
    registry
        .bindings_by_runtime_id
        .get(&runtime_id)
        .copied()
        .ok_or(HOST_STATUS_NOT_SUPPORTED)
}
