use crate::diagnostic::RuntimeStatus;
use crate::host::abi::background::{HostBackgroundEvent, decode_event};
use crate::host::android::abi::ingress::core::runtime_status;
use crate::host::android::ingress::android_notify_background_event;
use crate::host::core::error::invalid_argument_value;

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notify_background_event(
    runtime_id: u64,
    event: *const HostBackgroundEvent,
) -> RuntimeStatus {
    let result = if event.is_null() {
        Err(invalid_argument_value(
            "event",
            "event pointer must not be null",
        ))
    } else {
        let event = decode_event(unsafe { *event });
        event.and_then(|event| android_notify_background_event(runtime_id, event))
    };

    runtime_status(result)
}
