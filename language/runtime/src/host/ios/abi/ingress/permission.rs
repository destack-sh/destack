use crate::diagnostic::RuntimeStatus;
use crate::host::ios::ingress::ios_notify_permission_result;
use crate::runtime::NativeStringRef;

use super::core::{decode_string, runtime_status};

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_permission_result(
    runtime_id: u64,
    has_request_id: bool,
    request_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_string(permission, "permission").and_then(|permission| {
        let request_id = has_request_id.then_some(request_id);

        ios_notify_permission_result(runtime_id, request_id, permission.as_str(), granted)
    });

    runtime_status(result)
}
