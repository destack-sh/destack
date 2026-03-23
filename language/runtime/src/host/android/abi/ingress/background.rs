use crate::diagnostic::RuntimeStatus;
use crate::host::android::ingress::android_notify_background_event;
use crate::runtime::NativeSlice;

use super::core::{decode_background_event_payload, runtime_status};

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notify_background_event(
    runtime_id: u64,
    payload: NativeSlice<u8>,
) -> RuntimeStatus {
    let result =
        decode_background_event_payload(payload, "destack.host.android.notifyBackgroundEvent")
            .and_then(|event| android_notify_background_event(runtime_id, event));

    runtime_status(result)
}
