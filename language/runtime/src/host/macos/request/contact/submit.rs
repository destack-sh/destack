use super::query::{list_contacts, read_contact, search_contacts};
use super::write::{create_contact, delete_contact, update_contact};
use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};

/// Submit one macOS contact request through Contacts.
pub(crate) fn submit_contact_request(
    _context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // list contacts
        HostRequest::OsContactList { query } => {
            let page = list_contacts(query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }

        // search contacts
        HostRequest::OsContactSearch { query_text, query } => {
            let page = search_contacts(query_text, query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }

        // read one contact
        HostRequest::OsContactRead { id } => {
            let contact = read_contact(id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Contact(contact),
            )))
        }

        // create one contact
        HostRequest::OsContactCreate { contact } => {
            let id = create_contact(contact)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }

        // update one contact
        HostRequest::OsContactUpdate { id, contact } => {
            update_contact(id, contact)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // delete one contact
        HostRequest::OsContactDelete { id } => {
            delete_contact(id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
