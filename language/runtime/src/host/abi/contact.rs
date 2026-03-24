use crate::diagnostic::RuntimeResult;
use crate::host::abi::core::{HostOptionalStringRef, HostOptionalU32};
use crate::platform::NativeAbiCodec;
use crate::platform::os::abi_generated::{
    Contact, ContactDraft, ContactDraftValue, ContactPage, ContactPageValue, ContactQueryValue,
    ContactValue,
};
use crate::runtime::BindingCallContext;

/// One host contact query payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostContactQuery {
    /// The optional cursor field.
    pub cursor: HostOptionalStringRef,
    /// The optional page limit field.
    pub limit: HostOptionalU32,
    /// Whether phone numbers should be returned.
    pub include_phones: bool,
    /// Whether email addresses should be returned.
    pub include_emails: bool,
    /// Whether postal addresses should be returned.
    pub include_addresses: bool,
    /// Whether organization metadata should be returned.
    pub include_organization: bool,
    /// Whether note fields should be returned.
    pub include_notes: bool,
}

impl NativeAbiCodec for HostContactQuery {
    type Value = ContactQueryValue;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(ContactQueryValue {
            cursor: unsafe { self.cursor.into_value()? },
            limit: unsafe { self.limit.into_value()? },
            include_phones: self.include_phones,
            include_emails: self.include_emails,
            include_addresses: self.include_addresses,
            include_organization: self.include_organization,
            include_notes: self.include_notes,
        })
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        Self {
            cursor: HostOptionalStringRef::from_value(binding, value.cursor),
            limit: HostOptionalU32::from_value(binding, value.limit),
            include_phones: value.include_phones,
            include_emails: value.include_emails,
            include_addresses: value.include_addresses,
            include_organization: value.include_organization,
            include_notes: value.include_notes,
        }
    }
}

/// Encode one contact query payload for the host ABI.
pub(crate) fn encode_contact_query(
    binding: &BindingCallContext,
    query: &ContactQueryValue,
) -> HostContactQuery {
    <HostContactQuery as NativeAbiCodec>::from_value(binding, query.clone())
}

/// Encode one contact draft payload for the host ABI.
pub(crate) fn encode_contact_draft(
    binding: &BindingCallContext,
    draft: &ContactDraftValue,
) -> ContactDraft {
    <ContactDraft as NativeAbiCodec>::from_value(binding, draft.clone())
}

/// Decode one host contact page payload.
pub(crate) unsafe fn decode_contact_page(page: ContactPage) -> RuntimeResult<ContactPageValue> {
    unsafe { <ContactPage as NativeAbiCodec>::into_value(page) }
}

/// Decode one host contact payload.
pub(crate) unsafe fn decode_contact(contact: Contact) -> RuntimeResult<ContactValue> {
    unsafe { <Contact as NativeAbiCodec>::into_value(contact) }
}
