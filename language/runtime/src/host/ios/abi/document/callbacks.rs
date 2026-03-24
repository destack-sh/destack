use crate::host::abi::document::HostDocumentRequest;
use crate::host::ios::abi::bindings::invoke_ios_binding_callback;

/// Host callback for one iOS document pick request.
pub(crate) type IosHostDocumentPickCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostDocumentRequest) -> u32;

/// Callback table for iOS host document request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostDocumentCallbacks {
    /// Callback for `documentPick`.
    pub pick: Option<IosHostDocumentPickCallback>,
}

/// Resolve and invoke one iOS host document callback.
pub(super) fn call_ios_document_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostDocumentCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.document), invoke)
}
