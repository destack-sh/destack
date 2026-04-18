use crate::diagnostic::RuntimeResult;
use crate::host::abi::document::HostDocumentRequest;
use crate::host::core::callback::decode_callback_host_status;
use crate::host::os::android::abi::document::ffi::destack_host_android_document_pick;
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::NativeAbiCodec;
use crate::platform::abi::NativeStringSlice;
use crate::runtime::BindingCallContext;

/// Return one Android document request outcome when supported.
pub(crate) fn submit_document_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsDocumentPick { options } => {
            let binding = BindingCallContext::from_current_worker_for_native()?;
            let abi_request = HostDocumentRequest {
                request_id: context.request_id.0,
                mime_types: NativeStringSlice::from_value(&binding, options.mime_types.clone()),
                extensions: NativeStringSlice::from_value(&binding, options.extensions.clone()),
                multiple: <bool as NativeAbiCodec>::from_value(&binding, options.multiple),
                allow_directories: <bool as NativeAbiCodec>::from_value(
                    &binding,
                    options.allow_directories,
                ),
                copy_to_sandbox: <bool as NativeAbiCodec>::from_value(
                    &binding,
                    options.copy_to_sandbox,
                ),
            };
            let status = unsafe {
                destack_host_android_document_pick(context.host_session_id.0, abi_request)
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::event_completing(
                HostRequestResult::DocumentDescriptors(Vec::new()),
            )))
        }
        _ => Ok(None),
    }
}
