use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::callback::decode_callback_host_required_string_ref;
use crate::platform::PlatformError;
use crate::platform::os::abi_generated::{
    ContactAddressValue, ContactDraftValue, ContactEmailValue, ContactNameValue,
    ContactOrganizationValue, ContactPageValue, ContactPhoneValue, ContactQueryValue, ContactValue,
};
use crate::runtime::BindingCallContext;

use crate::host::abi::contact::{
    HostContact, HostContactAddress, HostContactCreateResponse, HostContactDraft, HostContactEmail,
    HostContactName, HostContactOrganization, HostContactPage, HostContactPageResponse,
    HostContactPhone, HostContactQuery, HostContactResponse,
};

/// Encode one contact query payload for the host ABI.
pub(crate) fn encode_contact_query(
    binding: &BindingCallContext,
    query: &ContactQueryValue,
) -> HostContactQuery {
    HostContactQuery {
        has_cursor: query.cursor.is_some(),
        cursor: binding.store_string(query.cursor.as_deref().unwrap_or("")),
        has_limit: query.limit.is_some(),
        limit: query.limit.unwrap_or_default(),
        include_phones: query.include_phones,
        include_emails: query.include_emails,
        include_addresses: query.include_addresses,
        include_organization: query.include_organization,
        include_notes: query.include_notes,
    }
}

/// Encode one contact draft payload for the host ABI.
pub(crate) fn encode_contact_draft(
    binding: &BindingCallContext,
    draft: &ContactDraftValue,
) -> HostContactDraft {
    HostContactDraft {
        name: encode_contact_name(binding, &draft.name),
        phones: binding.store_slice(
            draft
                .phones
                .iter()
                .map(|phone| encode_contact_phone(binding, phone))
                .collect(),
        ),
        emails: binding.store_slice(
            draft
                .emails
                .iter()
                .map(|email| encode_contact_email(binding, email))
                .collect(),
        ),
        addresses: binding.store_slice(
            draft
                .addresses
                .iter()
                .map(|address| encode_contact_address(binding, address))
                .collect(),
        ),
        organization: encode_contact_organization(binding, &draft.organization),
        note: binding.store_string(&draft.note),
    }
}

/// Decode one host contact-page response payload.
pub(crate) unsafe fn decode_contact_page_response(
    response: HostContactPageResponse,
    operation: &'static str,
) -> RuntimeResult<ContactPageValue> {
    if !response.has_page {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "response.page",
            format!("{operation} returned one missing response.page"),
        ))
        .boxed());
    }

    unsafe { decode_contact_page(response.page, operation) }
}

/// Decode one host contact-read response payload.
pub(crate) unsafe fn decode_contact_response(
    response: HostContactResponse,
    operation: &'static str,
) -> RuntimeResult<ContactValue> {
    if !response.has_contact {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "response.contact",
            format!("{operation} returned one missing response.contact"),
        ))
        .boxed());
    }

    unsafe { decode_contact(response.contact, operation) }
}

/// Decode one host contact-create response payload.
pub(crate) unsafe fn decode_contact_create_response(
    response: HostContactCreateResponse,
    operation: &'static str,
) -> RuntimeResult<String> {
    if !response.has_id {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "response.id",
            format!("{operation} returned one missing response.id"),
        ))
        .boxed());
    }

    decode_callback_host_required_string_ref(operation, "HostContactCreateResponse.id", response.id)
}

/// Encode one contact-name payload.
fn encode_contact_name(binding: &BindingCallContext, name: &ContactNameValue) -> HostContactName {
    HostContactName {
        given_name: binding.store_string(&name.given_name),
        middle_name: binding.store_string(&name.middle_name),
        family_name: binding.store_string(&name.family_name),
        prefix: binding.store_string(&name.prefix),
        suffix: binding.store_string(&name.suffix),
        nickname: binding.store_string(&name.nickname),
        phonetic_given_name: binding.store_string(&name.phonetic_given_name),
        phonetic_family_name: binding.store_string(&name.phonetic_family_name),
    }
}

/// Encode one contact-phone payload.
fn encode_contact_phone(
    binding: &BindingCallContext,
    phone: &ContactPhoneValue,
) -> HostContactPhone {
    HostContactPhone {
        label: binding.store_string(&phone.label),
        number: binding.store_string(&phone.number),
        normalized_number: binding.store_string(&phone.normalized_number),
        primary: phone.primary,
    }
}

/// Encode one contact-email payload.
fn encode_contact_email(
    binding: &BindingCallContext,
    email: &ContactEmailValue,
) -> HostContactEmail {
    HostContactEmail {
        label: binding.store_string(&email.label),
        address: binding.store_string(&email.address),
        primary: email.primary,
    }
}

/// Encode one contact-address payload.
fn encode_contact_address(
    binding: &BindingCallContext,
    address: &ContactAddressValue,
) -> HostContactAddress {
    HostContactAddress {
        label: binding.store_string(&address.label),
        street: binding.store_string(&address.street),
        city: binding.store_string(&address.city),
        region: binding.store_string(&address.region),
        postal_code: binding.store_string(&address.postal_code),
        country: binding.store_string(&address.country),
        country_code: binding.store_string(&address.country_code),
    }
}

/// Encode one contact-organization payload.
fn encode_contact_organization(
    binding: &BindingCallContext,
    organization: &ContactOrganizationValue,
) -> HostContactOrganization {
    HostContactOrganization {
        company: binding.store_string(&organization.company),
        department: binding.store_string(&organization.department),
        title: binding.store_string(&organization.title),
    }
}

/// Decode one host contact page.
unsafe fn decode_contact_page(
    page: HostContactPage,
    operation: &'static str,
) -> RuntimeResult<ContactPageValue> {
    let contacts = unsafe { page.contacts.as_slice()? };
    let contacts = contacts
        .iter()
        .copied()
        .map(|contact| unsafe { decode_contact(contact, operation) })
        .collect::<RuntimeResult<Vec<_>>>()?;

    let next_cursor = decode_callback_host_required_string_ref(
        operation,
        "HostContactPage.next_cursor",
        page.next_cursor,
    )?;

    Ok(ContactPageValue {
        contacts,
        next_cursor,
        has_more: page.has_more,
    })
}

/// Decode one host contact payload.
unsafe fn decode_contact(
    contact: HostContact,
    operation: &'static str,
) -> RuntimeResult<ContactValue> {
    let phones = unsafe { contact.phones.as_slice()? };
    let phones = phones
        .iter()
        .copied()
        .map(|phone| decode_contact_phone(phone, operation))
        .collect::<RuntimeResult<Vec<_>>>()?;

    let emails = unsafe { contact.emails.as_slice()? };
    let emails = emails
        .iter()
        .copied()
        .map(|email| decode_contact_email(email, operation))
        .collect::<RuntimeResult<Vec<_>>>()?;

    let addresses = unsafe { contact.addresses.as_slice()? };
    let addresses = addresses
        .iter()
        .copied()
        .map(|address| decode_contact_address(address, operation))
        .collect::<RuntimeResult<Vec<_>>>()?;

    Ok(ContactValue {
        id: decode_callback_host_required_string_ref(operation, "HostContact.id", contact.id)?,
        name: decode_contact_name(contact.name, operation)?,
        phones,
        emails,
        addresses,
        organization: decode_contact_organization(contact.organization, operation)?,
        note: decode_callback_host_required_string_ref(
            operation,
            "HostContact.note",
            contact.note,
        )?,
    })
}

/// Decode one host contact-name payload.
fn decode_contact_name(
    name: HostContactName,
    operation: &'static str,
) -> RuntimeResult<ContactNameValue> {
    Ok(ContactNameValue {
        given_name: decode_callback_host_required_string_ref(
            operation,
            "HostContactName.given_name",
            name.given_name,
        )?,
        middle_name: decode_callback_host_required_string_ref(
            operation,
            "HostContactName.middle_name",
            name.middle_name,
        )?,
        family_name: decode_callback_host_required_string_ref(
            operation,
            "HostContactName.family_name",
            name.family_name,
        )?,
        prefix: decode_callback_host_required_string_ref(
            operation,
            "HostContactName.prefix",
            name.prefix,
        )?,
        suffix: decode_callback_host_required_string_ref(
            operation,
            "HostContactName.suffix",
            name.suffix,
        )?,
        nickname: decode_callback_host_required_string_ref(
            operation,
            "HostContactName.nickname",
            name.nickname,
        )?,
        phonetic_given_name: decode_callback_host_required_string_ref(
            operation,
            "HostContactName.phonetic_given_name",
            name.phonetic_given_name,
        )?,
        phonetic_family_name: decode_callback_host_required_string_ref(
            operation,
            "HostContactName.phonetic_family_name",
            name.phonetic_family_name,
        )?,
    })
}

/// Decode one host contact-phone payload.
fn decode_contact_phone(
    phone: HostContactPhone,
    operation: &'static str,
) -> RuntimeResult<ContactPhoneValue> {
    Ok(ContactPhoneValue {
        label: decode_callback_host_required_string_ref(
            operation,
            "HostContactPhone.label",
            phone.label,
        )?,
        number: decode_callback_host_required_string_ref(
            operation,
            "HostContactPhone.number",
            phone.number,
        )?,
        normalized_number: decode_callback_host_required_string_ref(
            operation,
            "HostContactPhone.normalized_number",
            phone.normalized_number,
        )?,
        primary: phone.primary,
    })
}

/// Decode one host contact-email payload.
fn decode_contact_email(
    email: HostContactEmail,
    operation: &'static str,
) -> RuntimeResult<ContactEmailValue> {
    Ok(ContactEmailValue {
        label: decode_callback_host_required_string_ref(
            operation,
            "HostContactEmail.label",
            email.label,
        )?,
        address: decode_callback_host_required_string_ref(
            operation,
            "HostContactEmail.address",
            email.address,
        )?,
        primary: email.primary,
    })
}

/// Decode one host contact-address payload.
fn decode_contact_address(
    address: HostContactAddress,
    operation: &'static str,
) -> RuntimeResult<ContactAddressValue> {
    Ok(ContactAddressValue {
        label: decode_callback_host_required_string_ref(
            operation,
            "HostContactAddress.label",
            address.label,
        )?,
        street: decode_callback_host_required_string_ref(
            operation,
            "HostContactAddress.street",
            address.street,
        )?,
        city: decode_callback_host_required_string_ref(
            operation,
            "HostContactAddress.city",
            address.city,
        )?,
        region: decode_callback_host_required_string_ref(
            operation,
            "HostContactAddress.region",
            address.region,
        )?,
        postal_code: decode_callback_host_required_string_ref(
            operation,
            "HostContactAddress.postal_code",
            address.postal_code,
        )?,
        country: decode_callback_host_required_string_ref(
            operation,
            "HostContactAddress.country",
            address.country,
        )?,
        country_code: decode_callback_host_required_string_ref(
            operation,
            "HostContactAddress.country_code",
            address.country_code,
        )?,
    })
}

/// Decode one host contact-organization payload.
fn decode_contact_organization(
    organization: HostContactOrganization,
    operation: &'static str,
) -> RuntimeResult<ContactOrganizationValue> {
    Ok(ContactOrganizationValue {
        company: decode_callback_host_required_string_ref(
            operation,
            "HostContactOrganization.company",
            organization.company,
        )?,
        department: decode_callback_host_required_string_ref(
            operation,
            "HostContactOrganization.department",
            organization.department,
        )?,
        title: decode_callback_host_required_string_ref(
            operation,
            "HostContactOrganization.title",
            organization.title,
        )?,
    })
}
