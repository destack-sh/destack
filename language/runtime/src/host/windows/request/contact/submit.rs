use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};

use super::service::windows_contact_service;

/// Submit one Windows contact request through the shared WinRT contact service.
pub(crate) fn submit_contact_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let service = windows_contact_service(request.operation_name())?;

    match request {
        // list contacts
        HostRequest::OsContactList { query } => {
            let page = service.list_contacts(query, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }

        // search contacts
        HostRequest::OsContactSearch { query_text, query } => {
            let page = service.search_contacts(query_text, query, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }

        // read one contact
        HostRequest::OsContactRead { id } => {
            let contact = service.read_contact(id, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Contact(contact),
            )))
        }

        // create one contact
        HostRequest::OsContactCreate { contact } => {
            let id = service.create_contact(contact, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }

        // update one contact
        HostRequest::OsContactUpdate { id, contact } => {
            service.update_contact(id, contact, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // delete one contact
        HostRequest::OsContactDelete { id } => {
            service.delete_contact(id, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
