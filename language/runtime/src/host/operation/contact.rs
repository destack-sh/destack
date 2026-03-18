use super::{HostOperation, decode};

use crate::host::core::request::HostRequest;
use crate::platform::os::abi_generated::{
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue,
};

/// Build one contact list operation.
pub(crate) fn list(query: ContactQueryValue) -> HostOperation<ContactPageValue> {
    HostOperation::new(HostRequest::OsContactList { query }, decode::contact_page)
}

/// Build one contact search operation.
pub(crate) fn search(
    query_text: String,
    query: ContactQueryValue,
) -> HostOperation<ContactPageValue> {
    HostOperation::new(
        HostRequest::OsContactSearch { query_text, query },
        decode::contact_page,
    )
}

/// Build one contact read operation.
pub(crate) fn read(id: String) -> HostOperation<ContactValue> {
    HostOperation::new(HostRequest::OsContactRead { id }, decode::contact)
}

/// Build one contact create operation.
pub(crate) fn create(contact: ContactDraftValue) -> HostOperation<String> {
    HostOperation::new(HostRequest::OsContactCreate { contact }, decode::string)
}

/// Build one contact update operation.
pub(crate) fn update(id: String, contact: ContactDraftValue) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsContactUpdate { id, contact }, decode::none)
}

/// Build one contact delete operation.
pub(crate) fn delete(id: String) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsContactDelete { id }, decode::none)
}
