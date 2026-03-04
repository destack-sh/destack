use std::sync::{Arc, Mutex, OnceLock, Weak};

use super::core::AsioStreamRuntime;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

// NOTE #Architecture: ASIO callback entry points do not carry user-data context, so callback
// routing cannot recover a per-agent runtime-state handle, this slot remains process-global
/// One global callback runtime slot required by ASIO callback ABI.
static ACTIVE_ASIO_RUNTIME: OnceLock<Mutex<Option<Weak<AsioStreamRuntime>>>> = OnceLock::new();

/// Return one callback-runtime slot for ASIO callbacks.
fn active_runtime_slot() -> &'static Mutex<Option<Weak<AsioStreamRuntime>>> {
    ACTIVE_ASIO_RUNTIME.get_or_init(|| Mutex::new(None))
}

/// Install one active callback runtime.
pub(super) fn install_active_runtime(runtime: &Arc<AsioStreamRuntime>) -> RuntimeResult<()> {
    let mut slot = active_runtime_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // reject multiple simultaneous ASIO streams
    if let Some(active) = slot.as_ref().and_then(Weak::upgrade)
        && !Arc::ptr_eq(&active, runtime)
    {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open ASIO only supports one active stream",
        ))
        .boxed());
    }

    *slot = Some(Arc::downgrade(runtime));

    Ok(())
}

/// Return one upgraded active runtime when available.
pub(super) fn active_runtime() -> Option<Arc<AsioStreamRuntime>> {
    active_runtime_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .as_ref()
        .and_then(Weak::upgrade)
}

/// Clear one active runtime entry when it matches one runtime pointer.
pub(super) fn clear_active_runtime(runtime: *const AsioStreamRuntime) {
    let mut slot = active_runtime_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if let Some(active) = slot.as_ref().and_then(Weak::upgrade)
        && Arc::as_ptr(&active) == runtime
    {
        *slot = None;
    }
}
