use crate::diagnostic::RuntimeStatus;
use crate::host::android::ingress::android_notify_document_result;
use crate::platform::NativeSlice;
use crate::platform::os::abi_generated::DocumentDescriptorValue;

use super::core::runtime_status;

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notify_document_result(
    runtime_id: u64,
    request_id: u64,
    documents: NativeSlice<DocumentDescriptorValue>,
) -> RuntimeStatus {
    let result = unsafe { documents.as_slice() }
        .map(|documents| documents.to_vec())
        .and_then(|documents| android_notify_document_result(runtime_id, request_id, documents));

    runtime_status(result)
}
