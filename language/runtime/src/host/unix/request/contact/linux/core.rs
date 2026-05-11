use super::address_book::{
    create_contacts, get_contact, get_contact_list, modify_contacts, remove_contacts,
};
use super::page::{compare_contacts, paginate_contacts};
use super::vcard::{contact_value_from_vcard, search_contacts_query, vcard_text_from_draft};
use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::request::linux::eds::{
    EdsSourceDescriptor, EdsSourceKind, composite_eds_identifier, list_eds_sources,
    parse_composite_eds_identifier,
};
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue,
};
use crate::runtime::action::{HostAction, HostActionSet};

/// The contact list operation name.
const CONTACT_LIST_OPERATION: &str = "destack.os.contact.list";

/// The contact search operation name.
const CONTACT_SEARCH_OPERATION: &str = "destack.os.contact.search";

/// The contact read operation name.
const CONTACT_READ_OPERATION: &str = "destack.os.contact.read";

/// The contact create operation name.
const CONTACT_CREATE_OPERATION: &str = "destack.os.contact.create";

/// The contact update operation name.
const CONTACT_UPDATE_OPERATION: &str = "destack.os.contact.update";

/// The contact delete operation name.
const CONTACT_DELETE_OPERATION: &str = "destack.os.contact.delete";

/// Return dynamic Unix contact actions for Linux hosts.
pub(crate) fn request_actions() -> HostActionSet {
    let mut actions = HostActionSet::default();
    let Ok(sources) = list_eds_sources(EdsSourceKind::AddressBook, CONTACT_LIST_OPERATION) else {
        return actions;
    };

    // provider read support
    actions.insert_action(HostAction::OsContactRead);

    // writable address books
    if sources.iter().any(|source| source.is_writable) {
        actions.insert_action(HostAction::OsContactWrite);
    }

    actions
}

/// Submit one Linux contact request through EDS.
pub(crate) fn submit_contact_request(
    _context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // list one page of contacts
        HostRequest::OsContactList { query } => {
            let page = list_contacts(query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }

        // search contacts through one provider-side query
        HostRequest::OsContactSearch { query_text, query } => {
            let page = search_contacts(query_text, query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }

        // read one contact by stable identifier
        HostRequest::OsContactRead { id } => {
            let contact = read_contact(id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Contact(contact),
            )))
        }

        // create one contact in one writable address book
        HostRequest::OsContactCreate { contact } => {
            let id = create_contact(contact)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }

        // update one contact in its source address book
        HostRequest::OsContactUpdate { id, contact } => {
            update_contact(id, contact)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // delete one contact from its source address book
        HostRequest::OsContactDelete { id } => {
            delete_contact(id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// List contacts from all enabled address books.
fn list_contacts(query: &ContactQueryValue) -> RuntimeResult<ContactPageValue> {
    let sources = list_eds_sources(EdsSourceKind::AddressBook, CONTACT_LIST_OPERATION)?;
    let mut contacts = Vec::new();

    // provider source scan
    for source in sources {
        let vcards = get_contact_list(&source.uid, all_contacts_query(), CONTACT_LIST_OPERATION)?;

        // provider record materialization
        for vcard in vcards {
            let contact = contact_value_from_vcard(&source.uid, &vcard, query)?;
            contacts.push(contact);
        }
    }

    // stable ordering
    contacts.sort_by(compare_contacts);

    paginate_contacts(contacts, query)
}

/// Search contacts from all enabled address books.
fn search_contacts(query_text: &str, query: &ContactQueryValue) -> RuntimeResult<ContactPageValue> {
    let sources = list_eds_sources(EdsSourceKind::AddressBook, CONTACT_SEARCH_OPERATION)?;
    let mut contacts = Vec::new();
    let provider_query = search_contacts_query(query_text);

    // provider source scan
    for source in sources {
        let vcards = get_contact_list(&source.uid, &provider_query, CONTACT_SEARCH_OPERATION)?;

        // provider record materialization
        for vcard in vcards {
            let contact = contact_value_from_vcard(&source.uid, &vcard, query)?;
            contacts.push(contact);
        }
    }

    // stable ordering
    contacts.sort_by(compare_contacts);

    paginate_contacts(contacts, query)
}

/// Read one contact from its source address book.
fn read_contact(id: &str) -> RuntimeResult<ContactValue> {
    let (source_uid, contact_uid) =
        parse_composite_eds_identifier(id, CONTACT_READ_OPERATION, "id")?;
    let vcard = get_contact(&source_uid, &contact_uid, CONTACT_READ_OPERATION)?;
    let query = full_contact_query();

    contact_value_from_vcard(&source_uid, &vcard, &query)
}

/// Create one contact in one writable address book.
fn create_contact(draft: &ContactDraftValue) -> RuntimeResult<String> {
    let source = writable_contact_source(CONTACT_CREATE_OPERATION)?;
    let vcard = vcard_text_from_draft(draft, None);
    let ids = create_contacts(&source.uid, &[vcard], CONTACT_CREATE_OPERATION)?;
    let id = ids.into_iter().next().ok_or_else(|| {
        io_operation_error(
            CONTACT_CREATE_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            "evolution address book create returned no created contact identifiers",
        )
    })?;

    Ok(composite_eds_identifier(&source.uid, &id))
}

/// Update one existing contact in place.
fn update_contact(id: &str, draft: &ContactDraftValue) -> RuntimeResult<()> {
    let (source_uid, contact_uid) =
        parse_composite_eds_identifier(id, CONTACT_UPDATE_OPERATION, "id")?;

    ensure_writable_source(&source_uid, CONTACT_UPDATE_OPERATION)?;

    let vcard = vcard_text_from_draft(draft, Some(&contact_uid));
    modify_contacts(&source_uid, &[vcard], CONTACT_UPDATE_OPERATION)
}

/// Delete one existing contact.
fn delete_contact(id: &str) -> RuntimeResult<()> {
    let (source_uid, contact_uid) =
        parse_composite_eds_identifier(id, CONTACT_DELETE_OPERATION, "id")?;

    ensure_writable_source(&source_uid, CONTACT_DELETE_OPERATION)?;

    remove_contacts(&source_uid, &[contact_uid], CONTACT_DELETE_OPERATION)
}

/// Return one full-field query for read operations.
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

/// Return the EDS query that lists all contacts.
fn all_contacts_query() -> &'static str {
    r#"(contains "x-evolution-any-field" "")"#
}

/// Return one writable address-book source.
fn writable_contact_source(operation: &'static str) -> RuntimeResult<EdsSourceDescriptor> {
    let sources = list_eds_sources(EdsSourceKind::AddressBook, operation)?;

    sources
        .into_iter()
        .find(|source| source.is_writable)
        .ok_or_else(|| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoPermissionDenied),
                "no writable evolution address book source is available on this host",
            )
        })
}

/// Ensure one source remains writable for one mutating operation.
fn ensure_writable_source(source_uid: &str, operation: &'static str) -> RuntimeResult<()> {
    let sources = list_eds_sources(EdsSourceKind::AddressBook, operation)?;
    let source = sources
        .into_iter()
        .find(|source| source.uid == source_uid)
        .ok_or_else(|| io_not_found(operation, "evolution address book source was not found"))?;

    if source.is_writable {
        return Ok(());
    }

    Err(io_operation_error(
        operation,
        Some(PlatformErrorCode::IoPermissionDenied),
        "evolution address book source is not writable",
    ))
}
