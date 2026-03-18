use windows::ApplicationModel::Contacts::{
    Contact, ContactAddressKind, ContactEmailKind, ContactPhoneKind,
};
use windows::core::HSTRING;

use super::core::windows_contact_error;
use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{
    ContactAddressValue, ContactEmailValue, ContactNameValue, ContactOrganizationValue,
    ContactPhoneValue, ContactQueryValue, ContactValue,
};

/// Decode one native Windows contact into one runtime contact value.
pub(super) fn contact_value_from_native(
    contact: &Contact,
    query: &ContactQueryValue,
    operation: &'static str,
) -> RuntimeResult<ContactValue> {
    let id = contact
        .Id()
        .map_err(|error| windows_contact_error(operation, "Contact::Id", &error))?
        .to_string();
    let name = ContactNameValue {
        given_name: required_contact_string(contact.FirstName(), operation, "Contact::FirstName")?,
        middle_name: required_contact_string(
            contact.MiddleName(),
            operation,
            "Contact::MiddleName",
        )?,
        family_name: required_contact_string(contact.LastName(), operation, "Contact::LastName")?,
        prefix: required_contact_string(
            contact.HonorificNamePrefix(),
            operation,
            "Contact::HonorificNamePrefix",
        )?,
        suffix: required_contact_string(
            contact.HonorificNameSuffix(),
            operation,
            "Contact::HonorificNameSuffix",
        )?,
        nickname: required_contact_string(contact.Nickname(), operation, "Contact::Nickname")?,
        phonetic_given_name: required_contact_string(
            contact.YomiGivenName(),
            operation,
            "Contact::YomiGivenName",
        )?,
        phonetic_family_name: required_contact_string(
            contact.YomiFamilyName(),
            operation,
            "Contact::YomiFamilyName",
        )?,
    };
    let phones = if query.include_phones {
        phone_values_from_native(contact, operation)?
    } else {
        Vec::new()
    };
    let emails = if query.include_emails {
        email_values_from_native(contact, operation)?
    } else {
        Vec::new()
    };
    let addresses = if query.include_addresses {
        address_values_from_native(contact, operation)?
    } else {
        Vec::new()
    };
    let organization = if query.include_organization {
        organization_value_from_native(contact, operation)?
    } else {
        empty_organization()
    };
    let note = if query.include_notes {
        required_contact_string(contact.Notes(), operation, "Contact::Notes")?
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

/// Return the phone values for one native contact.
fn phone_values_from_native(
    contact: &Contact,
    operation: &'static str,
) -> RuntimeResult<Vec<ContactPhoneValue>> {
    let phones = contact
        .Phones()
        .map_err(|error| windows_contact_error(operation, "Contact::Phones", &error))?;
    let count = phones
        .Size()
        .map_err(|error| windows_contact_error(operation, "IVector::Size", &error))?;
    let mut values = Vec::with_capacity(count as usize);

    for index in 0..count {
        let phone = phones
            .GetAt(index)
            .map_err(|error| windows_contact_error(operation, "IVector::GetAt", &error))?;

        values.push(ContactPhoneValue {
            label: required_contact_string(
                phone.Description(),
                operation,
                "ContactPhone::Description",
            )?,
            number: required_contact_string(phone.Number(), operation, "ContactPhone::Number")?,
            normalized_number: required_contact_string(
                phone.Number(),
                operation,
                "ContactPhone::Number",
            )?,
            primary: index == 0,
        });
    }

    Ok(values)
}

/// Return the email values for one native contact.
fn email_values_from_native(
    contact: &Contact,
    operation: &'static str,
) -> RuntimeResult<Vec<ContactEmailValue>> {
    let emails = contact
        .Emails()
        .map_err(|error| windows_contact_error(operation, "Contact::Emails", &error))?;
    let count = emails
        .Size()
        .map_err(|error| windows_contact_error(operation, "IVector::Size", &error))?;
    let mut values = Vec::with_capacity(count as usize);

    for index in 0..count {
        let email = emails
            .GetAt(index)
            .map_err(|error| windows_contact_error(operation, "IVector::GetAt", &error))?;

        values.push(ContactEmailValue {
            label: required_contact_string(
                email.Description(),
                operation,
                "ContactEmail::Description",
            )?,
            address: required_contact_string(email.Address(), operation, "ContactEmail::Address")?,
            primary: index == 0,
        });
    }

    Ok(values)
}

/// Return the address values for one native contact.
fn address_values_from_native(
    contact: &Contact,
    operation: &'static str,
) -> RuntimeResult<Vec<ContactAddressValue>> {
    let addresses = contact
        .Addresses()
        .map_err(|error| windows_contact_error(operation, "Contact::Addresses", &error))?;
    let count = addresses
        .Size()
        .map_err(|error| windows_contact_error(operation, "IVector::Size", &error))?;
    let mut values = Vec::with_capacity(count as usize);

    for index in 0..count {
        let address = addresses
            .GetAt(index)
            .map_err(|error| windows_contact_error(operation, "IVector::GetAt", &error))?;

        values.push(ContactAddressValue {
            label: required_contact_string(
                address.Description(),
                operation,
                "ContactAddress::Description",
            )?,
            street: required_contact_string(
                address.StreetAddress(),
                operation,
                "ContactAddress::StreetAddress",
            )?,
            city: required_contact_string(
                address.Locality(),
                operation,
                "ContactAddress::Locality",
            )?,
            region: required_contact_string(address.Region(), operation, "ContactAddress::Region")?,
            postal_code: required_contact_string(
                address.PostalCode(),
                operation,
                "ContactAddress::PostalCode",
            )?,
            country: required_contact_string(
                address.Country(),
                operation,
                "ContactAddress::Country",
            )?,
            country_code: String::new(),
        });
    }

    Ok(values)
}

/// Return the organization value for one native contact.
fn organization_value_from_native(
    contact: &Contact,
    operation: &'static str,
) -> RuntimeResult<ContactOrganizationValue> {
    let jobs = contact
        .JobInfo()
        .map_err(|error| windows_contact_error(operation, "Contact::JobInfo", &error))?;
    let count = jobs
        .Size()
        .map_err(|error| windows_contact_error(operation, "IVector::Size", &error))?;

    if count == 0 {
        return Ok(empty_organization());
    }

    let job = jobs
        .GetAt(0)
        .map_err(|error| windows_contact_error(operation, "IVector::GetAt", &error))?;

    Ok(ContactOrganizationValue {
        company: required_contact_string(
            job.CompanyName(),
            operation,
            "ContactJobInfo::CompanyName",
        )?,
        department: required_contact_string(
            job.Department(),
            operation,
            "ContactJobInfo::Department",
        )?,
        title: required_contact_string(job.Title(), operation, "ContactJobInfo::Title")?,
    })
}

/// Return one required Windows contact string or one empty runtime string on missing data.
pub(super) fn required_contact_string(
    value: windows::core::Result<HSTRING>,
    operation: &'static str,
    stage: &str,
) -> RuntimeResult<String> {
    value
        .map(|value| value.to_string())
        .map_err(|error| windows_contact_error(operation, stage, &error))
}

/// Map one contact phone label to one Windows phone kind.
pub(super) fn contact_phone_kind(label: &str) -> ContactPhoneKind {
    let label = label.to_ascii_lowercase();

    if label.contains("mobile") || label.contains("cell") {
        return ContactPhoneKind::Mobile;
    }

    if label.contains("work") || label.contains("office") {
        return ContactPhoneKind::Work;
    }

    if label.contains("home") {
        return ContactPhoneKind::Home;
    }

    ContactPhoneKind::Other
}

/// Map one contact email label to one Windows email kind.
pub(super) fn contact_email_kind(label: &str) -> ContactEmailKind {
    let label = label.to_ascii_lowercase();

    if label.contains("work") || label.contains("office") {
        return ContactEmailKind::Work;
    }

    if label.contains("home") || label.contains("personal") {
        return ContactEmailKind::Personal;
    }

    ContactEmailKind::Other
}

/// Map one contact address label to one Windows address kind.
pub(super) fn contact_address_kind(label: &str) -> ContactAddressKind {
    let label = label.to_ascii_lowercase();

    if label.contains("work") || label.contains("office") {
        return ContactAddressKind::Work;
    }

    if label.contains("home") {
        return ContactAddressKind::Home;
    }

    ContactAddressKind::Other
}

/// Return one empty contact organization value.
pub(super) fn empty_organization() -> ContactOrganizationValue {
    ContactOrganizationValue {
        company: String::new(),
        department: String::new(),
        title: String::new(),
    }
}
