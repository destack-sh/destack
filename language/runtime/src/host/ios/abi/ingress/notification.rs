use crate::diagnostic::RuntimeStatus;
use crate::host::abi::notification::{HostNotificationEvent, decode_notification_event};
use crate::host::ios::ingress::ios_notify_notification_event;

use super::core::runtime_status;

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_notification_event(
    runtime_id: u64,
    event: HostNotificationEvent,
) -> RuntimeStatus {
    let result = decode_notification_event(event, "destack.host.ios.notifyNotificationEvent")
        .and_then(|event| ios_notify_notification_event(runtime_id, event));

    runtime_status(result)
}
