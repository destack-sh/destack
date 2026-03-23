use crate::diagnostic::RuntimeStatus;
use crate::host::ios::ingress::ios_notify_background_event;
use crate::runtime::NativeSlice;

use super::core::{decode_background_event_payload, runtime_status};

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_background_event(
    runtime_id: u64,
    payload: NativeSlice<u8>,
) -> RuntimeStatus {
    let result = decode_background_event_payload(payload, "destack.host.ios.notifyBackgroundEvent")
        .and_then(|event| ios_notify_background_event(runtime_id, event));

    runtime_status(result)
}
