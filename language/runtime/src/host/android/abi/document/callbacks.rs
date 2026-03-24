use crate::host::abi::document::HostDocumentRequest;
use crate::host::android::abi::bindings::invoke_android_binding_callback;

/// Host callback for one Android document pick request.
pub(crate) type AndroidHostDocumentPickCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostDocumentRequest) -> u32;

/// Callback table for Android host document request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostDocumentCallbacks {
    /// Callback for `documentPick`.
    pub pick: Option<AndroidHostDocumentPickCallback>,
}

/// Resolve and invoke one Android host document callback.
pub(super) fn call_android_document_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostDocumentCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.document), invoke)
}
