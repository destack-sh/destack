use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_contacts::{
    CNContactType, CNLabeledValue, CNMutableContact, CNMutablePostalAddress, CNPhoneNumber,
    CNPostalAddress,
};
use objc2_foundation::{NSArray, NSString};

use crate::platform::os::abi_generated::{
    ContactAddressValue, ContactDraftValue, ContactEmailValue, ContactPhoneValue,
};

/// Apply one runtime draft onto one mutable native contact.
pub(super) fn apply_contact_draft(contact: &CNMutableContact, draft: &ContactDraftValue) {
    let prefix = NSString::from_str(&draft.name.prefix);
    let given_name = NSString::from_str(&draft.name.given_name);
    let middle_name = NSString::from_str(&draft.name.middle_name);
    let family_name = NSString::from_str(&draft.name.family_name);
    let suffix = NSString::from_str(&draft.name.suffix);
    let nickname = NSString::from_str(&draft.name.nickname);
    let phonetic_given_name = NSString::from_str(&draft.name.phonetic_given_name);
    let phonetic_family_name = NSString::from_str(&draft.name.phonetic_family_name);
    let company = NSString::from_str(&draft.organization.company);
    let department = NSString::from_str(&draft.organization.department);
    let title = NSString::from_str(&draft.organization.title);
    let note = NSString::from_str(&draft.note);
    let phone_numbers = native_phone_values(&draft.phones);
    let email_addresses = native_email_values(&draft.emails);
    let postal_addresses = native_address_values(&draft.addresses);

    // scalar fields
    unsafe {
        contact.setContactType(if draft.organization.company.is_empty() {
            CNContactType::Person
        } else {
            CNContactType::Organization
        });
        contact.setNamePrefix(prefix.as_ref());
        contact.setGivenName(given_name.as_ref());
        contact.setMiddleName(middle_name.as_ref());
        contact.setFamilyName(family_name.as_ref());
        contact.setNameSuffix(suffix.as_ref());
        contact.setNickname(nickname.as_ref());
        contact.setPhoneticGivenName(phonetic_given_name.as_ref());
        contact.setPhoneticFamilyName(phonetic_family_name.as_ref());
        contact.setOrganizationName(company.as_ref());
        contact.setDepartmentName(department.as_ref());
        contact.setJobTitle(title.as_ref());
        contact.setNote(note.as_ref());
    }

    // collection fields
    unsafe {
        contact.setPhoneNumbers(phone_numbers.as_ref());
        contact.setEmailAddresses(email_addresses.as_ref());
        contact.setPostalAddresses(postal_addresses.as_ref());
    }
}

/// Build native phone values for one runtime draft.
fn native_phone_values(
    phones: &[ContactPhoneValue],
) -> Retained<NSArray<CNLabeledValue<CNPhoneNumber>>> {
    let native_values = phones
        .iter()
        .map(|phone| {
            let label = (!phone.label.is_empty()).then(|| NSString::from_str(&phone.label));
            let number_value = if phone.normalized_number.is_empty() {
                &phone.number
            } else {
                &phone.normalized_number
            };
            let number = NSString::from_str(number_value);
            let number = unsafe {
                CNPhoneNumber::initWithStringValue(CNPhoneNumber::alloc(), number.as_ref())
            };

            unsafe {
                CNLabeledValue::labeledValueWithLabel_value(label.as_deref(), number.as_ref())
            }
        })
        .collect::<Vec<_>>();

    NSArray::from_retained_slice(&native_values)
}

/// Build native email values for one runtime draft.
fn native_email_values(
    emails: &[ContactEmailValue],
) -> Retained<NSArray<CNLabeledValue<NSString>>> {
    let native_values = emails
        .iter()
        .map(|email| {
            let label = (!email.label.is_empty()).then(|| NSString::from_str(&email.label));
            let address = NSString::from_str(&email.address);

            unsafe {
                CNLabeledValue::labeledValueWithLabel_value(label.as_deref(), address.as_ref())
            }
        })
        .collect::<Vec<_>>();

    NSArray::from_retained_slice(&native_values)
}

/// Build native postal values for one runtime draft.
fn native_address_values(
    addresses: &[ContactAddressValue],
) -> Retained<NSArray<CNLabeledValue<CNPostalAddress>>> {
    let native_values = addresses
        .iter()
        .map(|address| {
            let label = (!address.label.is_empty()).then(|| NSString::from_str(&address.label));
            let street = NSString::from_str(&address.street);
            let city = NSString::from_str(&address.city);
            let region = NSString::from_str(&address.region);
            let postal_code = NSString::from_str(&address.postal_code);
            let country = NSString::from_str(&address.country);
            let country_code = NSString::from_str(&address.country_code);
            let postal_address = unsafe { CNMutablePostalAddress::new() };

            // postal fields
            unsafe {
                postal_address.setStreet(street.as_ref());
                postal_address.setCity(city.as_ref());
                postal_address.setState(region.as_ref());
                postal_address.setPostalCode(postal_code.as_ref());
                postal_address.setCountry(country.as_ref());
                postal_address.setISOCountryCode(country_code.as_ref());
            }

            unsafe {
                CNLabeledValue::labeledValueWithLabel_value(
                    label.as_deref(),
                    postal_address.as_ref(),
                )
            }
        })
        .collect::<Vec<_>>();

    NSArray::from_retained_slice(&native_values)
}
