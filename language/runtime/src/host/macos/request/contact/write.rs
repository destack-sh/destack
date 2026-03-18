use objc2_contacts::CNSaveRequest;
use objc2_foundation::NSMutableCopying;

use super::core::{
    CONTACT_CREATE_OPERATION, CONTACT_DELETE_OPERATION, CONTACT_UPDATE_OPERATION, contact_error,
    full_contact_query, native_contact_identifier, new_contact_store, new_mutable_contact,
};
use super::draft::apply_contact_draft;
use super::query::resolve_contact;
use crate::diagnostic::RuntimeResult;
use crate::host::apple::execution::call_process_main_context_if_needed;
use crate::platform::os::abi_generated::{ContactDraftValue, ContactQueryValue};

/// Create one contact in the default native container.
pub(super) fn create_contact(draft: &ContactDraftValue) -> RuntimeResult<String> {
    let draft = draft.clone();

    call_process_main_context_if_needed(move || {
        let store = new_contact_store();
        let contact = new_mutable_contact();

        // apply the runtime draft before saving
        apply_contact_draft(contact.as_ref(), &draft);

        let save_request = unsafe { CNSaveRequest::new() };

        // add the contact into the default container
        unsafe {
            save_request.addContact_toContainerWithIdentifier(contact.as_ref(), None);
        }

        unsafe { store.executeSaveRequest_error(save_request.as_ref()) }.map_err(|error| {
            contact_error(
                CONTACT_CREATE_OPERATION,
                format!("Contacts create failed: {error:?}"),
            )
        })?;

        Ok(native_contact_identifier(contact.as_ref()))
    })
}

/// Update one existing contact in place.
pub(super) fn update_contact(id: &str, draft: &ContactDraftValue) -> RuntimeResult<()> {
    let id = id.to_string();
    let draft = draft.clone();

    call_process_main_context_if_needed(move || {
        let store = new_contact_store();
        let query = full_contact_query();
        let native = resolve_contact(store.as_ref(), &id, &query, CONTACT_UPDATE_OPERATION)?;
        let contact = native.mutableCopy();

        // apply the runtime draft before saving
        apply_contact_draft(contact.as_ref(), &draft);

        let save_request = unsafe { CNSaveRequest::new() };

        // update the resolved contact
        unsafe {
            save_request.updateContact(contact.as_ref());
        }

        unsafe { store.executeSaveRequest_error(save_request.as_ref()) }.map_err(|error| {
            contact_error(
                CONTACT_UPDATE_OPERATION,
                format!("Contacts update failed: {error:?}"),
            )
        })?;

        Ok(())
    })
}

/// Delete one existing contact.
pub(super) fn delete_contact(id: &str) -> RuntimeResult<()> {
    let id = id.to_string();

    call_process_main_context_if_needed(move || {
        let store = new_contact_store();
        let query = ContactQueryValue {
            cursor: None,
            limit: None,
            include_phones: false,
            include_emails: false,
            include_addresses: false,
            include_organization: false,
            include_notes: false,
        };
        let native = resolve_contact(store.as_ref(), &id, &query, CONTACT_DELETE_OPERATION)?;
        let contact = native.mutableCopy();
        let save_request = unsafe { CNSaveRequest::new() };

        // delete the resolved contact
        unsafe {
            save_request.deleteContact(contact.as_ref());
        }

        unsafe { store.executeSaveRequest_error(save_request.as_ref()) }.map_err(|error| {
            contact_error(
                CONTACT_DELETE_OPERATION,
                format!("Contacts delete failed: {error:?}"),
            )
        })?;

        Ok(())
    })
}
