use crate::host::abi::contact::HostContactQuery;
use crate::host::android::abi::bindings::invoke_android_binding_callback;
use crate::platform::os::abi_generated::{Contact, ContactDraft, ContactPage};
use crate::runtime::NativeStringRef;

/// Host callback for listing Android contacts.
pub(crate) type AndroidHostContactListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    query: HostContactQuery,
    output_page: *mut ContactPage,
) -> u32;

/// Host callback for searching Android contacts.
pub(crate) type AndroidHostContactSearchCallback = unsafe extern "C" fn(
    runtime_id: u64,
    query_text: NativeStringRef,
    query: HostContactQuery,
    output_page: *mut ContactPage,
) -> u32;

/// Host callback for reading one Android contact.
pub(crate) type AndroidHostContactReadCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeStringRef, output_contact: *mut Contact) -> u32;

/// Host callback for creating one Android contact.
pub(crate) type AndroidHostContactCreateCallback = unsafe extern "C" fn(
    runtime_id: u64,
    draft: ContactDraft,
    output_id: *mut NativeStringRef,
) -> u32;

/// Host callback for updating one Android contact.
pub(crate) type AndroidHostContactUpdateCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeStringRef, draft: ContactDraft) -> u32;

/// Host callback for deleting one Android contact.
pub(crate) type AndroidHostContactDeleteCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeStringRef) -> u32;

/// Callback table for Android host contact request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostContactCallbacks {
    /// Callback for `contactList`.
    pub list: Option<AndroidHostContactListCallback>,
    /// Callback for `contactSearch`.
    pub search: Option<AndroidHostContactSearchCallback>,
    /// Callback for `contactRead`.
    pub read: Option<AndroidHostContactReadCallback>,
    /// Callback for `contactCreate`.
    pub create: Option<AndroidHostContactCreateCallback>,
    /// Callback for `contactUpdate`.
    pub update: Option<AndroidHostContactUpdateCallback>,
    /// Callback for `contactDelete`.
    pub delete: Option<AndroidHostContactDeleteCallback>,
}

/// Resolve and invoke one Android host contact callback.
pub(super) fn call_android_contact_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostContactCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.contact), invoke)
}
