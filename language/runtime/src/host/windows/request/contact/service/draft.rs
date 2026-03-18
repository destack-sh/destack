use windows::ApplicationModel::Contacts::{
    Contact, ContactAddress, ContactEmail, ContactJobInfo, ContactPhone,
};
use windows::core::HSTRING;

use super::core::windows_contact_error;
use super::native::{contact_address_kind, contact_email_kind, contact_phone_kind};
use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{
    ContactAddressValue, ContactDraftValue, ContactEmailValue, ContactOrganizationValue,
    ContactPhoneValue,
};

/// Replace one native Windows contact with the runtime draft fields.
pub(super) fn apply_contact_draft(
    contact: &Contact,
    draft: &ContactDraftValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    set_contact_string(
        &contact.SetFirstName(&HSTRING::from(&draft.name.given_name)),
        operation,
        "Contact::SetFirstName",
    )?;
    set_contact_string(
        &contact.SetMiddleName(&HSTRING::from(&draft.name.middle_name)),
        operation,
        "Contact::SetMiddleName",
    )?;
    set_contact_string(
        &contact.SetLastName(&HSTRING::from(&draft.name.family_name)),
        operation,
        "Contact::SetLastName",
    )?;
    set_contact_string(
        &contact.SetHonorificNamePrefix(&HSTRING::from(&draft.name.prefix)),
        operation,
        "Contact::SetHonorificNamePrefix",
    )?;
    set_contact_string(
        &contact.SetHonorificNameSuffix(&HSTRING::from(&draft.name.suffix)),
        operation,
        "Contact::SetHonorificNameSuffix",
    )?;
    set_contact_string(
        &contact.SetNickname(&HSTRING::from(&draft.name.nickname)),
        operation,
        "Contact::SetNickname",
    )?;
    set_contact_string(
        &contact.SetYomiGivenName(&HSTRING::from(&draft.name.phonetic_given_name)),
        operation,
        "Contact::SetYomiGivenName",
    )?;
    set_contact_string(
        &contact.SetYomiFamilyName(&HSTRING::from(&draft.name.phonetic_family_name)),
        operation,
        "Contact::SetYomiFamilyName",
    )?;
    set_contact_string(
        &contact.SetNotes(&HSTRING::from(&draft.note)),
        operation,
        "Contact::SetNotes",
    )?;

    replace_phone_values(contact, &draft.phones, operation)?;
    replace_email_values(contact, &draft.emails, operation)?;
    replace_address_values(contact, &draft.addresses, operation)?;
    replace_organization_value(contact, &draft.organization, operation)?;

    Ok(())
}

/// Replace the native phone vector for one contact.
fn replace_phone_values(
    contact: &Contact,
    values: &[ContactPhoneValue],
    operation: &'static str,
) -> RuntimeResult<()> {
    let phones = contact
        .Phones()
        .map_err(|error| windows_contact_error(operation, "Contact::Phones", &error))?;
    phones
        .Clear()
        .map_err(|error| windows_contact_error(operation, "IVector::Clear", &error))?;

    for value in values {
        let phone = ContactPhone::new()
            .map_err(|error| windows_contact_error(operation, "ContactPhone::new", &error))?;

        set_contact_string(
            &phone.SetDescription(&HSTRING::from(&value.label)),
            operation,
            "ContactPhone::SetDescription",
        )?;
        set_contact_string(
            &phone.SetNumber(&HSTRING::from(&value.number)),
            operation,
            "ContactPhone::SetNumber",
        )?;
        phone
            .SetKind(contact_phone_kind(&value.label))
            .map_err(|error| windows_contact_error(operation, "ContactPhone::SetKind", &error))?;
        phones
            .Append(&phone)
            .map_err(|error| windows_contact_error(operation, "IVector::Append", &error))?;
    }

    Ok(())
}

/// Replace the native email vector for one contact.
fn replace_email_values(
    contact: &Contact,
    values: &[ContactEmailValue],
    operation: &'static str,
) -> RuntimeResult<()> {
    let emails = contact
        .Emails()
        .map_err(|error| windows_contact_error(operation, "Contact::Emails", &error))?;
    emails
        .Clear()
        .map_err(|error| windows_contact_error(operation, "IVector::Clear", &error))?;

    for value in values {
        let email = ContactEmail::new()
            .map_err(|error| windows_contact_error(operation, "ContactEmail::new", &error))?;

        set_contact_string(
            &email.SetDescription(&HSTRING::from(&value.label)),
            operation,
            "ContactEmail::SetDescription",
        )?;
        set_contact_string(
            &email.SetAddress(&HSTRING::from(&value.address)),
            operation,
            "ContactEmail::SetAddress",
        )?;
        email
            .SetKind(contact_email_kind(&value.label))
            .map_err(|error| windows_contact_error(operation, "ContactEmail::SetKind", &error))?;
        emails
            .Append(&email)
            .map_err(|error| windows_contact_error(operation, "IVector::Append", &error))?;
    }

    Ok(())
}

/// Replace the native address vector for one contact.
fn replace_address_values(
    contact: &Contact,
    values: &[ContactAddressValue],
    operation: &'static str,
) -> RuntimeResult<()> {
    let addresses = contact
        .Addresses()
        .map_err(|error| windows_contact_error(operation, "Contact::Addresses", &error))?;
    addresses
        .Clear()
        .map_err(|error| windows_contact_error(operation, "IVector::Clear", &error))?;

    for value in values {
        let address = ContactAddress::new()
            .map_err(|error| windows_contact_error(operation, "ContactAddress::new", &error))?;

        set_contact_string(
            &address.SetDescription(&HSTRING::from(&value.label)),
            operation,
            "ContactAddress::SetDescription",
        )?;
        set_contact_string(
            &address.SetStreetAddress(&HSTRING::from(&value.street)),
            operation,
            "ContactAddress::SetStreetAddress",
        )?;
        set_contact_string(
            &address.SetLocality(&HSTRING::from(&value.city)),
            operation,
            "ContactAddress::SetLocality",
        )?;
        set_contact_string(
            &address.SetRegion(&HSTRING::from(&value.region)),
            operation,
            "ContactAddress::SetRegion",
        )?;
        set_contact_string(
            &address.SetPostalCode(&HSTRING::from(&value.postal_code)),
            operation,
            "ContactAddress::SetPostalCode",
        )?;
        set_contact_string(
            &address.SetCountry(&HSTRING::from(&value.country)),
            operation,
            "ContactAddress::SetCountry",
        )?;
        address
            .SetKind(contact_address_kind(&value.label))
            .map_err(|error| windows_contact_error(operation, "ContactAddress::SetKind", &error))?;
        addresses
            .Append(&address)
            .map_err(|error| windows_contact_error(operation, "IVector::Append", &error))?;
    }

    Ok(())
}

/// Replace the native organization vector for one contact.
fn replace_organization_value(
    contact: &Contact,
    value: &ContactOrganizationValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    let jobs = contact
        .JobInfo()
        .map_err(|error| windows_contact_error(operation, "Contact::JobInfo", &error))?;
    jobs.Clear()
        .map_err(|error| windows_contact_error(operation, "IVector::Clear", &error))?;

    if value.company.is_empty() && value.department.is_empty() && value.title.is_empty() {
        return Ok(());
    }

    let job = ContactJobInfo::new()
        .map_err(|error| windows_contact_error(operation, "ContactJobInfo::new", &error))?;

    set_contact_string(
        &job.SetCompanyName(&HSTRING::from(&value.company)),
        operation,
        "ContactJobInfo::SetCompanyName",
    )?;
    set_contact_string(
        &job.SetDepartment(&HSTRING::from(&value.department)),
        operation,
        "ContactJobInfo::SetDepartment",
    )?;
    set_contact_string(
        &job.SetTitle(&HSTRING::from(&value.title)),
        operation,
        "ContactJobInfo::SetTitle",
    )?;
    jobs.Append(&job)
        .map_err(|error| windows_contact_error(operation, "IVector::Append", &error))?;

    Ok(())
}

/// Propagate one Windows setter result through the shared contact error mapper.
fn set_contact_string(
    result: &windows::core::Result<()>,
    operation: &'static str,
    stage: &str,
) -> RuntimeResult<()> {
    result
        .as_ref()
        .map_err(|error| windows_contact_error(operation, stage, error))?;

    Ok(())
}
