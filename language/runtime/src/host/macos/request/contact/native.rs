use objc2_contacts::CNContact;

use crate::platform::os::abi_generated::{
    ContactAddressValue, ContactEmailValue, ContactNameValue, ContactOrganizationValue,
    ContactPhoneValue,
};

/// Materialize one runtime contact name payload.
pub(super) fn contact_name_from_native(contact: &CNContact) -> ContactNameValue {
    ContactNameValue {
        given_name: unsafe { contact.givenName() }.to_string(),
        middle_name: unsafe { contact.middleName() }.to_string(),
        family_name: unsafe { contact.familyName() }.to_string(),
        prefix: unsafe { contact.namePrefix() }.to_string(),
        suffix: unsafe { contact.nameSuffix() }.to_string(),
        nickname: unsafe { contact.nickname() }.to_string(),
        phonetic_given_name: unsafe { contact.phoneticGivenName() }.to_string(),
        phonetic_family_name: unsafe { contact.phoneticFamilyName() }.to_string(),
    }
}

/// Materialize runtime phone payloads from one native contact.
pub(super) fn contact_phones_from_native(contact: &CNContact) -> Vec<ContactPhoneValue> {
    let native_values = unsafe { contact.phoneNumbers() };
    let mut phones = Vec::with_capacity(native_values.len());

    // preserve the native order as the best available primary signal
    for (index, value) in native_values.iter().enumerate() {
        let label = unsafe { value.label() }
            .map(|value| value.to_string())
            .unwrap_or_default();
        let number_value = unsafe { value.value() };
        let number = unsafe { number_value.stringValue() }.to_string();

        phones.push(ContactPhoneValue {
            label,
            normalized_number: number.clone(),
            number,
            primary: index == 0,
        });
    }

    phones
}

/// Materialize runtime email payloads from one native contact.
pub(super) fn contact_emails_from_native(contact: &CNContact) -> Vec<ContactEmailValue> {
    let native_values = unsafe { contact.emailAddresses() };
    let mut emails = Vec::with_capacity(native_values.len());

    // preserve the native order as the best available primary signal
    for (index, value) in native_values.iter().enumerate() {
        let label = unsafe { value.label() }
            .map(|value| value.to_string())
            .unwrap_or_default();
        let address = unsafe { value.value() }.to_string();

        emails.push(ContactEmailValue {
            label,
            address,
            primary: index == 0,
        });
    }

    emails
}

/// Materialize runtime postal payloads from one native contact.
pub(super) fn contact_addresses_from_native(contact: &CNContact) -> Vec<ContactAddressValue> {
    let native_values = unsafe { contact.postalAddresses() };
    let mut addresses = Vec::with_capacity(native_values.len());

    // map each native labeled postal address directly
    for value in native_values.iter() {
        let label = unsafe { value.label() }
            .map(|value| value.to_string())
            .unwrap_or_default();
        let address = unsafe { value.value() };

        addresses.push(ContactAddressValue {
            label,
            street: unsafe { address.street() }.to_string(),
            city: unsafe { address.city() }.to_string(),
            region: unsafe { address.state() }.to_string(),
            postal_code: unsafe { address.postalCode() }.to_string(),
            country: unsafe { address.country() }.to_string(),
            country_code: unsafe { address.ISOCountryCode() }.to_string(),
        });
    }

    addresses
}

/// Materialize runtime organization metadata from one native contact.
pub(super) fn contact_organization_from_native(contact: &CNContact) -> ContactOrganizationValue {
    ContactOrganizationValue {
        company: unsafe { contact.organizationName() }.to_string(),
        department: unsafe { contact.departmentName() }.to_string(),
        title: unsafe { contact.jobTitle() }.to_string(),
    }
}
