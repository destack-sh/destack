use std::sync::Arc;

use destack_artifact::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::{HostQueue, HostSessionHandle, HostSessionId, HostSessionRegistry};

/// Return the active iOS host queue for this process.
pub(crate) fn ios_host_queue(session_handle: HostSessionHandle) -> RuntimeResult<Arc<HostQueue>> {
    let host_session_id = HostSessionId::from_handle(session_handle);

    HostSessionRegistry::queue_for_session(host_session_id, Platform::IOS)
}
