use windows::ApplicationModel::Contacts::{
    Contact, ContactBatchStatus, ContactList, ContactQueryDesiredFields, ContactQueryOptions,
    ContactQuerySearchFields, ContactStore, ContactStoreAccessType,
};
use windows::core::HSTRING;

use super::core::windows_contact_error;
use super::native::contact_value_from_native;
use crate::diagnostic::RuntimeResult;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{ContactQueryValue, ContactValue};

/// Return one contact store with the requested access level.
pub(super) fn request_contact_store(
    access_type: ContactStoreAccessType,
    operation: &'static str,
) -> RuntimeResult<ContactStore> {
    windows::ApplicationModel::Contacts::ContactManager::RequestStoreAsyncWithAccessType(
        access_type,
    )
    .map_err(|error| {
        windows_contact_error(
            operation,
            "ContactManager::RequestStoreAsyncWithAccessType",
            &error,
        )
    })?
    .get()
    .map_err(|error| windows_contact_error(operation, "IAsyncOperation::get", &error))
}

/// Return one contact query with all runtime fields enabled.
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

/// Return one writable contact list for new contacts.
pub(super) fn default_contact_list(
    store: &ContactStore,
    operation: &'static str,
) -> RuntimeResult<ContactList> {
    let contact_lists = store
        .FindContactListsAsync()
        .map_err(|error| {
            windows_contact_error(operation, "ContactStore::FindContactListsAsync", &error)
        })?
        .get()
        .map_err(|error| windows_contact_error(operation, "IAsyncOperation::get", &error))?;
    let count = contact_lists
        .Size()
        .map_err(|error| windows_contact_error(operation, "IVectorView::Size", &error))?;

    // reuse one existing writable list before creating runtime-owned storage
    if count > 0 {
        return contact_lists
            .GetAt(0)
            .map_err(|error| windows_contact_error(operation, "IVectorView::GetAt", &error));
    }

    store
        .CreateContactListAsync(&HSTRING::from("Destack"))
        .map_err(|error| {
            windows_contact_error(operation, "ContactStore::CreateContactListAsync", &error)
        })?
        .get()
        .map_err(|error| windows_contact_error(operation, "IAsyncOperation::get", &error))
}

/// Return the writable contact list that owns one existing contact.
pub(super) fn contact_list_for_contact(
    store: &ContactStore,
    contact: &Contact,
    operation: &'static str,
) -> RuntimeResult<ContactList> {
    let contact_list_id = contact
        .ContactListId()
        .map_err(|error| windows_contact_error(operation, "Contact::ContactListId", &error))?;

    if contact_list_id.is_empty() {
        return default_contact_list(store, operation);
    }

    store
        .GetContactListAsync(&contact_list_id)
        .map_err(|error| {
            windows_contact_error(operation, "ContactStore::GetContactListAsync", &error)
        })?
        .get()
        .map_err(|error| windows_contact_error(operation, "IAsyncOperation::get", &error))
}

/// Read contacts for one optional search query.
pub(super) fn read_contacts(
    store: &ContactStore,
    query_text: Option<&str>,
    query: &ContactQueryValue,
    operation: &'static str,
) -> RuntimeResult<Vec<ContactValue>> {
    let options = query_options(query_text, query, operation)?;
    let reader = store
        .GetContactReaderWithOptions(&options)
        .map_err(|error| {
            windows_contact_error(
                operation,
                "ContactStore::GetContactReaderWithOptions",
                &error,
            )
        })?;
    let mut contacts = Vec::new();

    loop {
        let batch = reader
            .ReadBatchAsync()
            .map_err(|error| {
                windows_contact_error(operation, "ContactReader::ReadBatchAsync", &error)
            })?
            .get()
            .map_err(|error| windows_contact_error(operation, "IAsyncOperation::get", &error))?;
        let status = batch
            .Status()
            .map_err(|error| windows_contact_error(operation, "ContactBatch::Status", &error))?;

        if status != ContactBatchStatus::Success {
            return Err(io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!(
                    "windows contact reader returned one failed batch status: {}",
                    status.0
                ),
            ));
        }

        let batch_contacts = batch
            .Contacts()
            .map_err(|error| windows_contact_error(operation, "ContactBatch::Contacts", &error))?;
        let batch_size = batch_contacts
            .Size()
            .map_err(|error| windows_contact_error(operation, "IVectorView::Size", &error))?;

        if batch_size == 0 {
            break;
        }

        // decode one runtime contact per native row
        for index in 0..batch_size {
            let native_contact = batch_contacts
                .GetAt(index)
                .map_err(|error| windows_contact_error(operation, "IVectorView::GetAt", &error))?;
            let contact = contact_value_from_native(&native_contact, query, operation)?;

            contacts.push(contact);
        }
    }

    Ok(contacts)
}

/// Build one Windows contact query option set for the requested field projection.
fn query_options(
    query_text: Option<&str>,
    query: &ContactQueryValue,
    operation: &'static str,
) -> RuntimeResult<ContactQueryOptions> {
    let options = if let Some(query_text) = query_text {
        let query_text = HSTRING::from(query_text);

        ContactQueryOptions::CreateWithTextAndFields(&query_text, ContactQuerySearchFields::All)
            .map_err(|error| {
                windows_contact_error(
                    operation,
                    "ContactQueryOptions::CreateWithTextAndFields",
                    &error,
                )
            })?
    } else {
        ContactQueryOptions::new()
            .map_err(|error| windows_contact_error(operation, "ContactQueryOptions::new", &error))?
    };

    let mut desired_fields = ContactQueryDesiredFields::None;

    if query.include_phones {
        desired_fields |= ContactQueryDesiredFields::PhoneNumber;
    }

    if query.include_emails {
        desired_fields |= ContactQueryDesiredFields::EmailAddress;
    }

    if query.include_addresses {
        desired_fields |= ContactQueryDesiredFields::PostalAddress;
    }

    options.SetDesiredFields(desired_fields).map_err(|error| {
        windows_contact_error(operation, "ContactQueryOptions::SetDesiredFields", &error)
    })?;

    Ok(options)
}

/// Return one contact by identifier or fail loudly.
pub(super) fn get_contact(
    store: &ContactStore,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<Contact> {
    let id = HSTRING::from(id);

    store
        .GetContactAsync(&id)
        .map_err(|error| windows_contact_error(operation, "ContactStore::GetContactAsync", &error))?
        .get()
        .map_err(|error| windows_contact_error(operation, "IAsyncOperation::get", &error))
}
