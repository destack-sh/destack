use crate::host::abi::text::{HostTextGeometryRequest, HostTextOpenRequest, HostTextStateRequest};
use crate::host::android::abi::bindings::invoke_android_binding_callback;

/// Host callback for opening one Android text session.
pub(crate) type AndroidHostTextOpenCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostTextOpenRequest) -> u32;

/// Host callback for closing one Android text session.
pub(crate) type AndroidHostTextCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;

/// Host callback for updating one Android text geometry payload.
pub(crate) type AndroidHostTextSetGeometryCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostTextGeometryRequest) -> u32;

/// Host callback for updating one Android text state.
pub(crate) type AndroidHostTextSetStateCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostTextStateRequest) -> u32;

/// Callback table for Android host text request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostTextCallbacks {
    /// Callback for `textOpen`.
    pub open: Option<AndroidHostTextOpenCallback>,
    /// Callback for `textClose`.
    pub close: Option<AndroidHostTextCloseCallback>,
    /// Callback for `textSetGeometry`.
    pub set_geometry: Option<AndroidHostTextSetGeometryCallback>,
    /// Callback for `textSetState`.
    pub set_state: Option<AndroidHostTextSetStateCallback>,
}

/// Resolve and invoke one Android host text callback.
pub(super) fn call_android_text_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostTextCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.text), invoke)
}
