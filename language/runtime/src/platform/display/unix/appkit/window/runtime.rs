use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;

use super::super::model::AppKitWindowBinding;

/// Enforce owner-thread affinity for one opened window handle.
pub(crate) fn ensure_window_thread(
    binding: &AppKitWindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let current_thread_id = std::thread::current().id();

    // accept reads from the owner thread only
    if binding.owner_thread_id == current_thread_id {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        "handle",
        format!("{operation} must run on the owner thread of this window"),
    ))
}
