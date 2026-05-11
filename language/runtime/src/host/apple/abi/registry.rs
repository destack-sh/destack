#![cfg_attr(not(target_os = "ios"), allow(dead_code))]

use std::sync::OnceLock;

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use super::binding::IosHostBindings;
use crate::host::{HostSessionHandle, HostSessionId, HostSessionRegistry, HostStatus, Platform};

/// Shared iOS bindings registry state.
#[derive(Debug, Default)]
struct IosBindingsRegistryState {
    /// Runtime-scoped callback bindings keyed by runtime id.
    bindings_by_session_handle: FxHashMap<HostSessionHandle, IosHostBindings>,
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
fn has_ios_host_bridge(session_handle: HostSessionHandle) -> bool {
    let host_session_id = HostSessionId::from_handle(session_handle);

    HostSessionRegistry::queue_for_session(host_session_id, Platform::IOS).is_ok()
}

/// Register one runtime-scoped iOS bindings payload.
pub(crate) fn register_ios_bindings(
    session_handle: HostSessionHandle,
    bindings: IosHostBindings,
) -> u32 {
    // reject unknown runtime ids up front
    if !has_ios_host_bridge(session_handle) {
        return HostStatus::NotFound.code();
    }

    // insert one runtime-scoped bindings payload exactly once
    let mut registry = ios_bindings_registry().write();
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

/// Remove one runtime-scoped iOS bindings payload.
pub(crate) fn unregister_ios_bindings(host_session_id: HostSessionId) {
    // remove one runtime-scoped bindings payload when present
    let mut registry = ios_bindings_registry().write();
    registry
        .bindings_by_session_handle
        .remove(&host_session_id.0);
}

/// Resolve one runtime-scoped iOS bindings snapshot.
pub(crate) fn resolve_ios_bindings(
    session_handle: HostSessionHandle,
) -> Result<IosHostBindings, u32> {
    // drop stale entries when the runtime state is already gone
    if !has_ios_host_bridge(session_handle) {
        let mut registry = ios_bindings_registry().write();
        registry.bindings_by_session_handle.remove(&session_handle);
        return Err(HostStatus::NotFound.code());
    }

    // return one stable snapshot or report missing host registration
    let registry = ios_bindings_registry().read();
    registry
        .bindings_by_session_handle
        .get(&session_handle)
        .copied()
        .ok_or(HostStatus::NotSupported.code())
}
