use objc2::rc::Retained;
use objc2_contacts::{CNContactStore, CNMutableContact};
use objc2_foundation::NSString;

use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::ContactQueryValue;

/// The contact list operation name.
pub(super) const CONTACT_LIST_OPERATION: &str = "destack.os.contact.list";

/// The contact search operation name.
pub(super) const CONTACT_SEARCH_OPERATION: &str = "destack.os.contact.search";

/// The contact read operation name.
pub(super) const CONTACT_READ_OPERATION: &str = "destack.os.contact.read";

/// The contact create operation name.
pub(super) const CONTACT_CREATE_OPERATION: &str = "destack.os.contact.create";

/// The contact update operation name.
pub(super) const CONTACT_UPDATE_OPERATION: &str = "destack.os.contact.update";

/// The contact delete operation name.
pub(super) const CONTACT_DELETE_OPERATION: &str = "destack.os.contact.delete";

/// Return one full contact query with every supported field enabled.
pub(super) fn full_contact_query() -> ContactQueryValue {
    ContactQueryValue {
        cursor: None,
        limit: None,
        include_phones: true,
        include_emails: true,
        include_addresses: true,
        include_organization: true,
        include_notes: true,
    }
}

/// Create one fresh native contact store.
pub(super) fn new_contact_store() -> Retained<CNContactStore> {
    unsafe { CNContactStore::new() }
}

/// Build one empty mutable native contact.
pub(super) fn new_mutable_contact() -> Retained<CNMutableContact> {
    unsafe { CNMutableContact::new() }
}

/// Return one native contact identifier string.
pub(super) fn native_contact_identifier(contact: &CNMutableContact) -> String {
    let id = unsafe { contact.identifier() };

    id.to_string()
}

/// Build one native NSString value.
pub(super) fn ns_string(value: &str) -> Retained<NSString> {
    NSString::from_str(value)
}

/// Build one loud Contacts backend error.
pub(super) fn contact_error(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("{operation}: {}", message.into()),
    ))
    .boxed()
}
