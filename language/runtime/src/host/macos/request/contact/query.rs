use objc2::AnyThread;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_contacts::{
    CNContact, CNContactDepartmentNameKey, CNContactEmailAddressesKey, CNContactFamilyNameKey,
    CNContactFetchRequest, CNContactGivenNameKey, CNContactIdentifierKey, CNContactJobTitleKey,
    CNContactMiddleNameKey, CNContactNamePrefixKey, CNContactNameSuffixKey, CNContactNicknameKey,
    CNContactNoteKey, CNContactOrganizationNameKey, CNContactPhoneNumbersKey,
    CNContactPhoneticFamilyNameKey, CNContactPhoneticGivenNameKey, CNContactPostalAddressesKey,
    CNContactSortOrder, CNContactStore,
};
use objc2_foundation::{NSArray, NSPredicate};
use rustc_hash::FxHashSet;

use super::core::{
    CONTACT_LIST_OPERATION, CONTACT_READ_OPERATION, CONTACT_SEARCH_OPERATION, contact_error,
    full_contact_query, new_contact_store, ns_string,
};
use super::native::{
    contact_addresses_from_native, contact_emails_from_native, contact_name_from_native,
    contact_organization_from_native, contact_phones_from_native,
};
use super::page::{append_unique_contacts, paginate_contacts, sort_contacts};
use crate::diagnostic::RuntimeResult;
use crate::host::apple::core::execution::call_process_main_context_if_needed;
use crate::platform::os::abi_generated::{
    ContactOrganizationValue, ContactPageValue, ContactQueryValue, ContactValue,
};

/// List contacts through one paged fetch request.
pub(super) fn list_contacts(query: &ContactQueryValue) -> RuntimeResult<ContactPageValue> {
    let query = query.clone();

    call_process_main_context_if_needed(move || {
        let store = new_contact_store();
        let contacts = fetch_contacts(store.as_ref(), None, &query, CONTACT_LIST_OPERATION)?;

        Ok(paginate_contacts(contacts, &query))
    })
}

/// Search contacts through one set of native predicates.
pub(super) fn search_contacts(
    query_text: &str,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    let query_text = query_text.to_string();
    let query = query.clone();

    call_process_main_context_if_needed(move || {
        let store = new_contact_store();
        let mut contacts = Vec::new();
        let mut seen_ids = FxHashSet::default();

        // search by displayable name
        let name_query = ns_string(&query_text);
        let name_predicate =
            unsafe { CNContact::predicateForContactsMatchingName(name_query.as_ref()) };
        append_unique_contacts(
            &mut contacts,
            &mut seen_ids,
            fetch_contacts(
                store.as_ref(),
                Some(name_predicate.as_ref()),
                &query,
                CONTACT_SEARCH_OPERATION,
            )?,
        );

        // search by email address when the input looks like one
        if query_text.contains('@') {
            let email_query = ns_string(&query_text);
            let email_predicate = unsafe {
                CNContact::predicateForContactsMatchingEmailAddress(email_query.as_ref())
            };
            append_unique_contacts(
                &mut contacts,
                &mut seen_ids,
                fetch_contacts(
                    store.as_ref(),
                    Some(email_predicate.as_ref()),
                    &query,
                    CONTACT_SEARCH_OPERATION,
                )?,
            );
        }

        // search by phone number when the input contains digits
        if query_text.bytes().any(|byte| byte.is_ascii_digit()) {
            let phone_query = ns_string(&query_text);

            if let Some(phone_number) = unsafe {
                objc2_contacts::CNPhoneNumber::phoneNumberWithStringValue(phone_query.as_ref())
            } {
                let phone_predicate = unsafe {
                    CNContact::predicateForContactsMatchingPhoneNumber(phone_number.as_ref())
                };
                append_unique_contacts(
                    &mut contacts,
                    &mut seen_ids,
                    fetch_contacts(
                        store.as_ref(),
                        Some(phone_predicate.as_ref()),
                        &query,
                        CONTACT_SEARCH_OPERATION,
                    )?,
                );
            }
        }

        // stable ordering before paging
        sort_contacts(&mut contacts);

        Ok(paginate_contacts(contacts, &query))
    })
}

/// Read one contact by identifier with the full runtime field set.
pub(super) fn read_contact(id: &str) -> RuntimeResult<ContactValue> {
    let id = id.to_string();

    call_process_main_context_if_needed(move || {
        let store = new_contact_store();
        let query = full_contact_query();
        let native = resolve_contact(store.as_ref(), &id, &query, CONTACT_READ_OPERATION)?;

        contact_value_from_native(native.as_ref(), &query)
    })
}

/// Fetch contacts through one optional predicate and one selected key set.
pub(super) fn fetch_contacts(
    store: &CNContactStore,
    predicate: Option<&NSPredicate>,
    query: &ContactQueryValue,
    operation: &'static str,
) -> RuntimeResult<Vec<ContactValue>> {
    let keys = contact_keys(query);
    let request = unsafe {
        CNContactFetchRequest::initWithKeysToFetch(CNContactFetchRequest::alloc(), keys.as_ref())
    };

    // keep unified results in one stable user-visible order
    unsafe {
        request.setPredicate(predicate);
        request.setUnifyResults(true);
        request.setSortOrder(CNContactSortOrder::UserDefault);
    }

    let fetch_result = unsafe { store.enumeratorForContactFetchRequest_error(request.as_ref()) }
        .map_err(|error| contact_error(operation, format!("Contacts fetch failed: {error:?}")))?;
    let enumerator = unsafe { fetch_result.value() };
    let mut contacts = Vec::new();

    // materialize one runtime contact per native result
    while let Some(contact) = enumerator.nextObject() {
        let contact = contact_value_from_native(contact.as_ref(), query)?;
        contacts.push(contact);
    }

    Ok(contacts)
}

/// Resolve one contact by stable identifier.
pub(super) fn resolve_contact(
    store: &CNContactStore,
    id: &str,
    query: &ContactQueryValue,
    operation: &'static str,
) -> RuntimeResult<Retained<CNContact>> {
    let keys = contact_keys(query);
    let identifier = ns_string(id);

    unsafe {
        store.unifiedContactWithIdentifier_keysToFetch_error(identifier.as_ref(), keys.as_ref())
    }
    .map_err(|error| contact_error(operation, format!("Contacts resolve failed: {error:?}")))
}

/// Build the selected Contacts key list for one runtime query.
pub(super) fn contact_keys(
    query: &ContactQueryValue,
) -> Retained<NSArray<ProtocolObject<dyn objc2_contacts::CNKeyDescriptor>>> {
    let mut keys = vec![
        unsafe { ProtocolObject::from_ref(CNContactIdentifierKey) },
        unsafe { ProtocolObject::from_ref(CNContactGivenNameKey) },
        unsafe { ProtocolObject::from_ref(CNContactMiddleNameKey) },
        unsafe { ProtocolObject::from_ref(CNContactFamilyNameKey) },
        unsafe { ProtocolObject::from_ref(CNContactNamePrefixKey) },
        unsafe { ProtocolObject::from_ref(CNContactNameSuffixKey) },
        unsafe { ProtocolObject::from_ref(CNContactNicknameKey) },
        unsafe { ProtocolObject::from_ref(CNContactPhoneticGivenNameKey) },
        unsafe { ProtocolObject::from_ref(CNContactPhoneticFamilyNameKey) },
    ];

    // requested phone payloads
    if query.include_phones {
        keys.push(unsafe { ProtocolObject::from_ref(CNContactPhoneNumbersKey) });
    }

    // requested email payloads
    if query.include_emails {
        keys.push(unsafe { ProtocolObject::from_ref(CNContactEmailAddressesKey) });
    }

    // requested postal payloads
    if query.include_addresses {
        keys.push(unsafe { ProtocolObject::from_ref(CNContactPostalAddressesKey) });
    }

    // requested organization payloads
    if query.include_organization {
        keys.push(unsafe { ProtocolObject::from_ref(CNContactOrganizationNameKey) });
        keys.push(unsafe { ProtocolObject::from_ref(CNContactDepartmentNameKey) });
        keys.push(unsafe { ProtocolObject::from_ref(CNContactJobTitleKey) });
    }

    // requested note payload
    if query.include_notes {
        keys.push(unsafe { ProtocolObject::from_ref(CNContactNoteKey) });
    }

    NSArray::from_slice(&keys)
}

/// Materialize one runtime contact value from one native contact.
pub(super) fn contact_value_from_native(
    contact: &CNContact,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactValue> {
    let id = unsafe { contact.identifier() }.to_string();
    let name = contact_name_from_native(contact);
    let phones = if query.include_phones {
        contact_phones_from_native(contact)
    } else {
        Vec::new()
    };
    let emails = if query.include_emails {
        contact_emails_from_native(contact)
    } else {
        Vec::new()
    };
    let addresses = if query.include_addresses {
        contact_addresses_from_native(contact)
    } else {
        Vec::new()
    };
    let organization = if query.include_organization {
        contact_organization_from_native(contact)
    } else {
        ContactOrganizationValue {
            company: String::new(),
            department: String::new(),
            title: String::new(),
        }
    };
    let note = if query.include_notes {
        unsafe { contact.note() }.to_string()
    } else {
        String::new()
    };

    Ok(ContactValue {
        id,
        name,
        phones,
        emails,
        addresses,
        organization,
        note,
    })
}
