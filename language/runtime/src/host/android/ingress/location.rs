use crate::diagnostic::RuntimeResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{HostEvent, HostLocationEvent};
use crate::platform::os::LocationSampleValue;

/// Submit one Android location callback.
pub(crate) fn android_notify_location_sample(
    session_handle: HostSessionHandle,
    watch_id: &str,
    sample: LocationSampleValue,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;

    queue.enqueue(HostEvent::Location(Box::new(HostLocationEvent {
        watch_id: watch_id.to_string(),
        sample,
    })));

    Ok(())
}
