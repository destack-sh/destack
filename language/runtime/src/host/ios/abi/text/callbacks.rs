use crate::host::abi::text::{HostTextGeometryRequest, HostTextOpenRequest, HostTextStateRequest};
use crate::host::ios::abi::bindings::invoke_ios_binding_callback;

/// Host callback for opening one iOS text session.
pub(crate) type IosHostTextOpenCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostTextOpenRequest) -> u32;

/// Host callback for closing one iOS text session.
pub(crate) type IosHostTextCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;

/// Host callback for updating one iOS text geometry payload.
pub(crate) type IosHostTextSetGeometryCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostTextGeometryRequest) -> u32;

/// Host callback for updating one iOS text state.
pub(crate) type IosHostTextSetStateCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostTextStateRequest) -> u32;

/// Callback table for iOS host text request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostTextCallbacks {
    /// Callback for `textOpen`.
    pub open: Option<IosHostTextOpenCallback>,
    /// Callback for `textClose`.
    pub close: Option<IosHostTextCloseCallback>,
    /// Callback for `textSetGeometry`.
    pub set_geometry: Option<IosHostTextSetGeometryCallback>,
    /// Callback for `textSetState`.
    pub set_state: Option<IosHostTextSetStateCallback>,
}

/// Resolve and invoke one iOS host text callback.
pub(super) fn call_ios_text_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostTextCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.text), invoke)
}
