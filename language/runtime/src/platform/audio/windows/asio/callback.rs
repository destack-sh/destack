use std::sync::{Arc, Mutex, Weak};

use super::core::AsioStreamRuntime;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy};

// NOTE #Architecture: ASIO callback entry points do not carry user-data context, so callback
// routing cannot recover a per-worker runtime-state handle, this service remains process-global
/// One process-global ASIO callback routing service.
struct AsioCallbackService {
    /// The currently active ASIO runtime for callback dispatch.
    active_runtime: Mutex<Option<Weak<AsioStreamRuntime>>>,
}

impl AsioCallbackService {
    /// Build one empty ASIO callback routing service.
    fn new() -> Self {
        Self {
            active_runtime: Mutex::new(None),
        }
    }
}

impl Service for AsioCallbackService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return the shared ASIO callback routing service.
fn asio_callback_service() -> Arc<AsioCallbackService> {
    AsioCallbackService::global(|| Ok(AsioCallbackService::new()))
        .expect("ASIO callback service initialization should not fail")
}

/// Install one active callback runtime in the shared service.
pub(super) fn install_active_runtime(runtime: &Arc<AsioStreamRuntime>) -> RuntimeResult<()> {
    let service = asio_callback_service();
    let mut active_runtime = service
        .active_runtime
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // reject multiple simultaneous ASIO streams
    if let Some(active) = active_runtime.as_ref().and_then(Weak::upgrade)
        && !Arc::ptr_eq(&active, runtime)
    {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open ASIO only supports one active stream",
        ))
        .boxed());
    }

    *active_runtime = Some(Arc::downgrade(runtime));

    Ok(())
}

/// Return one upgraded active runtime from the shared service when available.
pub(super) fn active_runtime() -> Option<Arc<AsioStreamRuntime>> {
    let service = asio_callback_service();
    service
        .active_runtime
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .as_ref()
        .and_then(Weak::upgrade)
}

/// Clear one active runtime entry when it matches one runtime pointer.
pub(super) fn clear_active_runtime(runtime: *const AsioStreamRuntime) {
    let service = asio_callback_service();
    let mut active_runtime = service
        .active_runtime
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if let Some(active) = active_runtime.as_ref().and_then(Weak::upgrade)
        && Arc::as_ptr(&active) == runtime
    {
        *active_runtime = None;
    }
}
