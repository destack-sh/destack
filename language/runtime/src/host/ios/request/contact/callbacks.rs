use crate::host::ios::bridge::bindings::invoke_ios_binding_callback;
use crate::runtime::NativeSlice;

/// Host callback for listing iOS contacts.
pub(crate) type IosHostContactListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for searching iOS contacts.
pub(crate) type IosHostContactSearchCallback = unsafe extern "C" fn(
    runtime_id: u64,
    query_text: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for reading one iOS contact.
pub(crate) type IosHostContactReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    id: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for creating one iOS contact.
pub(crate) type IosHostContactCreateCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for updating one iOS contact.
pub(crate) type IosHostContactUpdateCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>, payload: NativeSlice<u8>) -> u32;

/// Host callback for deleting one iOS contact.
pub(crate) type IosHostContactDeleteCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeSlice<u8>) -> u32;

/// Callback table for iOS host contact request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostContactCallbacks {
    /// Callback for `contactList`.
    pub list: Option<IosHostContactListCallback>,
    /// Callback for `contactSearch`.
    pub search: Option<IosHostContactSearchCallback>,
    /// Callback for `contactRead`.
    pub read: Option<IosHostContactReadCallback>,
    /// Callback for `contactCreate`.
    pub create: Option<IosHostContactCreateCallback>,
    /// Callback for `contactUpdate`.
    pub update: Option<IosHostContactUpdateCallback>,
    /// Callback for `contactDelete`.
    pub delete: Option<IosHostContactDeleteCallback>,
}

/// Resolve and invoke one iOS host contact callback.
pub(super) fn call_ios_contact_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostContactCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.contact), invoke)
}
