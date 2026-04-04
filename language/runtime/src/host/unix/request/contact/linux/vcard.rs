use vobject::{Component, Property, Vcard, write_component};

use super::core::CONTACT_READ_OPERATION;
use super::page::{
    compare_contacts, display_contact_name, paginate_contacts, parse_contact_cursor,
};
use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::request::linux::eds::composite_eds_identifier;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    ContactAddressValue, ContactDraftValue, ContactEmailValue, ContactNameValue,
    ContactOrganizationValue, ContactPhoneValue, ContactQueryValue, ContactValue,
};

/// Return one escaped EDS any-field query string.
pub(super) fn search_contacts_query(query_text: &str) -> String {
    let query_text = query_text.replace('\\', r#"\\"#).replace('"', r#"\""#);

    format!(r#"(contains "x-evolution-any-field" "{query_text}")"#)
}

/// Materialize one runtime contact from one provider vcard.
pub(super) fn contact_value_from_vcard(
    source_uid: &str,
    vcard: &Vcard,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactValue> {
    let contact_uid = required_vcard_value(vcard, "UID", CONTACT_READ_OPERATION)?;
    let id = composite_eds_identifier(source_uid, &contact_uid);
    let name = contact_name_from_vcard(vcard);
    let phones = if query.include_phones {
        contact_phones_from_vcard(vcard)
    } else {
        Vec::new()
    };
    let emails = if query.include_emails {
        contact_emails_from_vcard(vcard)
    } else {
        Vec::new()
    };
    let addresses = if query.include_addresses {
        contact_addresses_from_vcard(vcard)
    } else {
        Vec::new()
    };
    let organization = if query.include_organization {
        contact_organization_from_vcard(vcard)
    } else {
        empty_contact_organization()
    };
    let note = if query.include_notes {
        optional_vcard_value(vcard, "NOTE").unwrap_or_default()
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

/// Build one serialized vcard from one runtime contact draft.
pub(super) fn vcard_text_from_draft(
    draft: &ContactDraftValue,
    existing_uid: Option<&str>,
) -> String {
    let mut component = Component::new("VCARD");

    component.set(Property::new("VERSION", "3.0"));

    if let Some(existing_uid) = existing_uid {
        component.set(Property::new("UID", existing_uid));
    }

    // name
    component.set(property_with_structured_values(
        "N",
        &[
            &draft.name.family_name,
            &draft.name.given_name,
            &draft.name.middle_name,
            &draft.name.prefix,
            &draft.name.suffix,
        ],
    ));

    let full_name = display_contact_name(&draft.name, &draft.organization);
    component.set(Property::new("FN", full_name));

    if !draft.name.nickname.is_empty() {
        component.set(Property::new("NICKNAME", &draft.name.nickname));
    }

    if !draft.name.phonetic_given_name.is_empty() {
        component.set(Property::new(
            "X-PHONETIC-FIRST-NAME",
            &draft.name.phonetic_given_name,
        ));
    }

    if !draft.name.phonetic_family_name.is_empty() {
        component.set(Property::new(
            "X-PHONETIC-LAST-NAME",
            &draft.name.phonetic_family_name,
        ));
    }

    // phones
    for phone in &draft.phones {
        component.push(labeled_property(
            "TEL",
            &phone.number,
            &phone.label,
            phone.primary,
        ));
    }

    // emails
    for email in &draft.emails {
        component.push(labeled_property(
            "EMAIL",
            &email.address,
            &email.label,
            email.primary,
        ));
    }

    // addresses
    for address in &draft.addresses {
        let mut property = labeled_property("ADR", "", &address.label, false);

        property.raw_value = structured_vcard_value(&[
            "",
            "",
            &address.street,
            &address.city,
            &address.region,
            &address.postal_code,
            &address.country,
        ]);

        if !address.country_code.is_empty() {
            property
                .params
                .insert("X-COUNTRYCODE".to_string(), address.country_code.clone());
        }

        component.push(property);
    }

    // organization
    if !draft.organization.company.is_empty() || !draft.organization.department.is_empty() {
        component.set(property_with_structured_values(
            "ORG",
            &[&draft.organization.company, &draft.organization.department],
        ));
    }

    if !draft.organization.title.is_empty() {
        component.set(Property::new("TITLE", &draft.organization.title));
    }

    // note
    if !draft.note.is_empty() {
        component.set(Property::new("NOTE", &draft.note));
    }

    write_component(&component)
}

/// Return one structured contact name from one provider vcard.
pub(super) fn contact_name_from_vcard(vcard: &Vcard) -> ContactNameValue {
    let values = structured_vcard_values(vcard.get_only("N"), 5);

    ContactNameValue {
        given_name: values[1].clone(),
        middle_name: values[2].clone(),
        family_name: values[0].clone(),
        prefix: values[3].clone(),
        suffix: values[4].clone(),
        nickname: optional_vcard_value(vcard, "NICKNAME").unwrap_or_default(),
        phonetic_given_name: optional_vcard_value(vcard, "X-PHONETIC-FIRST-NAME")
            .unwrap_or_default(),
        phonetic_family_name: optional_vcard_value(vcard, "X-PHONETIC-LAST-NAME")
            .unwrap_or_default(),
    }
}

/// Return one runtime phone list from one provider vcard.
pub(super) fn contact_phones_from_vcard(vcard: &Vcard) -> Vec<ContactPhoneValue> {
    let mut phones = Vec::new();

    // phone materialization
    for property in vcard.get_all("TEL") {
        let number = property.value_as_string();

        phones.push(ContactPhoneValue {
            label: property_label(property),
            number: number.clone(),
            normalized_number: normalized_phone_number(&number),
            primary: property_is_primary(property),
        });
    }

    phones
}

/// Return one runtime email list from one provider vcard.
pub(super) fn contact_emails_from_vcard(vcard: &Vcard) -> Vec<ContactEmailValue> {
    let mut emails = Vec::new();

    // email materialization
    for property in vcard.get_all("EMAIL") {
        emails.push(ContactEmailValue {
            label: property_label(property),
            address: property.value_as_string(),
            primary: property_is_primary(property),
        });
    }

    emails
}

/// Return one runtime address list from one provider vcard.
pub(super) fn contact_addresses_from_vcard(vcard: &Vcard) -> Vec<ContactAddressValue> {
    let mut addresses = Vec::new();

    // address materialization
    for property in vcard.get_all("ADR") {
        let values = structured_property_values(property, 7);

        addresses.push(ContactAddressValue {
            label: property_label(property),
            street: values[2].clone(),
            city: values[3].clone(),
            region: values[4].clone(),
            postal_code: values[5].clone(),
            country: values[6].clone(),
            country_code: property
                .params
                .get("X-COUNTRYCODE")
                .cloned()
                .unwrap_or_default(),
        });
    }

    addresses
}

/// Return one runtime organization payload from one provider vcard.
pub(super) fn contact_organization_from_vcard(vcard: &Vcard) -> ContactOrganizationValue {
    let values = structured_vcard_values(vcard.get_only("ORG"), 2);

    ContactOrganizationValue {
        company: values[0].clone(),
        department: values[1].clone(),
        title: optional_vcard_value(vcard, "TITLE").unwrap_or_default(),
    }
}

/// Return one empty organization payload.
fn empty_contact_organization() -> ContactOrganizationValue {
    ContactOrganizationValue {
        company: String::new(),
        department: String::new(),
        title: String::new(),
    }
}

/// Return one required scalar vcard property value.
fn required_vcard_value(
    vcard: &Vcard,
    property_name: &str,
    operation: &'static str,
) -> RuntimeResult<String> {
    optional_vcard_value(vcard, property_name).ok_or_else(|| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("evolution address book returned one contact without `{property_name}`"),
        )
    })
}

/// Return one optional scalar vcard property value.
fn optional_vcard_value(vcard: &Vcard, property_name: &str) -> Option<String> {
    vcard.get_only(property_name).map(Property::value_as_string)
}

/// Return one structured vcard property value list.
fn structured_vcard_values(property: Option<&Property>, fields: usize) -> Vec<String> {
    let Some(property) = property else {
        return vec![String::new(); fields];
    };

    structured_property_values(property, fields)
}

/// Return one structured property value list.
fn structured_property_values(property: &Property, fields: usize) -> Vec<String> {
    let mut values = split_structured_vcard_value(&property.raw_value)
        .into_iter()
        .map(|value| vobject::unescape_chars(&value))
        .collect::<Vec<_>>();

    if values.len() < fields {
        values.resize(fields, String::new());
    }

    values.truncate(fields);
    values
}

/// Split one structured raw vcard value on unescaped semicolons.
fn split_structured_vcard_value(raw_value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    // raw value scan
    for character in raw_value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        if character == '\\' {
            current.push(character);
            escaped = true;
            continue;
        }

        if character == ';' {
            values.push(current);
            current = String::new();
            continue;
        }

        current.push(character);
    }

    values.push(current);

    values
}

/// Build one structured property with one vcard-escaped semicolon list.
fn property_with_structured_values(name: &str, values: &[&str]) -> Property {
    let mut property = Property::new(name, "");
    property.raw_value = structured_vcard_value(values);

    property
}

/// Encode one structured vcard raw value.
fn structured_vcard_value(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| vobject::escape_chars(value))
        .collect::<Vec<_>>()
        .join(";")
}

/// Build one labeled property payload.
fn labeled_property(name: &str, value: &str, label: &str, is_primary: bool) -> Property {
    let mut property = Property::new(name, value);
    let mut type_tokens = Vec::new();

    if !label.is_empty() {
        type_tokens.push(label.to_ascii_uppercase());
    }

    if is_primary {
        type_tokens.push("PREF".to_string());
    }

    if !type_tokens.is_empty() {
        property
            .params
            .insert("TYPE".to_string(), type_tokens.join(","));
    }

    property
}

/// Return one normalized label string from one property.
fn property_label(property: &Property) -> String {
    if let Some(label) = property.params.get("X-ABLabel") {
        let label = label.trim();

        if !label.is_empty() {
            return label.to_ascii_lowercase();
        }
    }

    let Some(type_value) = property.params.get("TYPE") else {
        return String::new();
    };

    for token in type_value.split(',') {
        let token = token.trim().trim_matches('"');
        let normalized = token.to_ascii_lowercase();

        if normalized.is_empty()
            || matches!(
                normalized.as_str(),
                "pref" | "internet" | "voice" | "postal" | "parcel" | "intl" | "dom"
            )
        {
            continue;
        }

        return normalized;
    }

    String::new()
}

/// Return whether one property is marked as preferred.
fn property_is_primary(property: &Property) -> bool {
    if let Some(pref_value) = property.params.get("PREF") {
        let pref_value = pref_value.trim().trim_matches('"').to_ascii_lowercase();

        if pref_value == "1" || pref_value == "true" {
            return true;
        }
    }

    let Some(type_value) = property.params.get("TYPE") else {
        return false;
    };

    type_value
        .split(',')
        .any(|token| token.trim().eq_ignore_ascii_case("pref"))
}

/// Return one normalized phone-number payload when available.
fn normalized_phone_number(number: &str) -> String {
    let mut normalized = String::with_capacity(number.len());
    let mut has_leading_plus = false;

    // normalization
    for (index, character) in number.chars().enumerate() {
        if character.is_ascii_digit() {
            normalized.push(character);
            continue;
        }

        if index == 0 && character == '+' {
            normalized.push(character);
            has_leading_plus = true;
        }
    }

    if normalized == "+" || (!has_leading_plus && normalized.is_empty()) {
        return String::new();
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::{
        compare_contacts, contact_name_from_vcard, contact_organization_from_vcard,
        contact_value_from_vcard, display_contact_name, paginate_contacts, parse_contact_cursor,
        property_is_primary, property_label, search_contacts_query, split_structured_vcard_value,
        structured_vcard_value, vcard_text_from_draft,
    };
    use crate::platform::os::abi_generated::{
        ContactDraftValue, ContactEmailValue, ContactNameValue, ContactOrganizationValue,
        ContactPhoneValue, ContactQueryValue, ContactValue,
    };
    use vobject::{Property, Vcard};

    #[test]
    fn test_search_contacts_query_escapes_quotes_and_backslashes() {
        let query = search_contacts_query(r#"Ada "Dev" \ Team"#);

        assert_eq!(
            query,
            r#"(contains "x-evolution-any-field" "Ada \"Dev\" \\ Team")"#
        );
    }

    #[test]
    fn test_split_structured_vcard_value_preserves_escaped_semicolons() {
        let values = split_structured_vcard_value(r#"one\;two;three;four"#);

        assert_eq!(values, vec![r#"one\;two"#, "three", "four"]);
    }

    #[test]
    fn test_vcard_draft_roundtrips_structured_fields() {
        let draft = sample_contact_draft();
        let text = vcard_text_from_draft(&draft, Some("uid-1"));
        let vcard = Vcard::build(&text).unwrap();
        let query = full_query();
        let contact = contact_value_from_vcard("source-1", &vcard, &query).unwrap();

        assert_eq!(contact.name, draft.name);
        assert_eq!(contact.phones.len(), 1);
        assert_eq!(contact.emails.len(), 1);
        assert_eq!(contact.addresses.len(), 0);
        assert_eq!(contact.organization, draft.organization);
        assert_eq!(contact.note, draft.note);
    }

    #[test]
    fn test_contact_name_from_vcard_reads_structured_name_fields() {
        let vcard = Vcard::build(
            "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:test\r\nN:Doe;Ada;Lovelace;Countess;III\r\nNICKNAME:Analyst\r\nEND:VCARD\r\n",
        )
        .unwrap();
        let name = contact_name_from_vcard(&vcard);

        assert_eq!(name.family_name, "Doe");
        assert_eq!(name.given_name, "Ada");
        assert_eq!(name.middle_name, "Lovelace");
        assert_eq!(name.prefix, "Countess");
        assert_eq!(name.suffix, "III");
        assert_eq!(name.nickname, "Analyst");
    }

    #[test]
    fn test_contact_organization_from_vcard_reads_company_and_department() {
        let vcard = Vcard::build(
            "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:test\r\nORG:Analytical Engines;Research\r\nTITLE:Programmer\r\nEND:VCARD\r\n",
        )
        .unwrap();
        let organization = contact_organization_from_vcard(&vcard);

        assert_eq!(organization.company, "Analytical Engines");
        assert_eq!(organization.department, "Research");
        assert_eq!(organization.title, "Programmer");
    }

    #[test]
    fn test_property_label_prefers_non_pref_types() {
        let mut property = Property::new("TEL", "+41 44 123 45 67");
        property
            .params
            .insert("TYPE".to_string(), "PREF,WORK,VOICE".to_string());

        assert_eq!(property_label(&property), "work");
        assert!(property_is_primary(&property));
    }

    #[test]
    fn test_paginate_contacts_uses_decimal_offsets() {
        let contacts = vec![
            sample_contact("contact-1", "Ada"),
            sample_contact("contact-2", "Grace"),
            sample_contact("contact-3", "Linus"),
        ];
        let query = ContactQueryValue {
            cursor: Some("1".to_string()),
            limit: Some(1),
            include_phones: false,
            include_emails: false,
            include_addresses: false,
            include_organization: false,
            include_notes: false,
        };
        let page = paginate_contacts(contacts, &query).unwrap();

        assert_eq!(page.contacts, vec![sample_contact("contact-2", "Grace")]);
        assert_eq!(page.next_cursor, "2");
        assert!(page.has_more);
    }

    #[test]
    fn test_parse_contact_cursor_rejects_invalid_offsets() {
        let error = parse_contact_cursor(Some("oops")).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("cursor"));
    }

    #[test]
    fn test_compare_contacts_uses_display_name_then_id() {
        let mut contacts = vec![
            sample_contact("contact-2", "Grace"),
            sample_contact("contact-1", "Ada"),
        ];

        contacts.sort_by(compare_contacts);

        assert_eq!(
            contacts,
            vec![
                sample_contact("contact-1", "Ada"),
                sample_contact("contact-2", "Grace")
            ]
        );
    }

    #[test]
    fn test_display_contact_name_falls_back_to_company() {
        let name = ContactNameValue {
            given_name: String::new(),
            middle_name: String::new(),
            family_name: String::new(),
            prefix: String::new(),
            suffix: String::new(),
            nickname: String::new(),
            phonetic_given_name: String::new(),
            phonetic_family_name: String::new(),
        };
        let organization = ContactOrganizationValue {
            company: "Analytical Engines".to_string(),
            department: String::new(),
            title: String::new(),
        };

        assert_eq!(
            display_contact_name(&name, &organization),
            "Analytical Engines"
        );
    }

    #[test]
    fn test_structured_vcard_value_escapes_semicolons() {
        let value = structured_vcard_value(&["Ada;Grace", "Lovelace"]);

        assert_eq!(value, r#"Ada\;Grace;Lovelace"#);
    }

    fn full_query() -> ContactQueryValue {
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

    fn sample_contact_draft() -> ContactDraftValue {
        ContactDraftValue {
            name: ContactNameValue {
                given_name: "Ada".to_string(),
                middle_name: "Lovelace".to_string(),
                family_name: "Byron".to_string(),
                prefix: String::new(),
                suffix: String::new(),
                nickname: "Analyst".to_string(),
                phonetic_given_name: String::new(),
                phonetic_family_name: String::new(),
            },
            phones: vec![ContactPhoneValue {
                label: "work".to_string(),
                number: "+41 44 123 45 67".to_string(),
                normalized_number: "+41441234567".to_string(),
                primary: true,
            }],
            emails: vec![ContactEmailValue {
                label: "work".to_string(),
                address: "ada@example.com".to_string(),
                primary: true,
            }],
            addresses: Vec::new(),
            organization: ContactOrganizationValue {
                company: "Analytical Engines".to_string(),
                department: "Research".to_string(),
                title: "Programmer".to_string(),
            },
            note: "first programmer".to_string(),
        }
    }

    fn sample_contact(id: &str, given_name: &str) -> ContactValue {
        ContactValue {
            id: id.to_string(),
            name: ContactNameValue {
                given_name: given_name.to_string(),
                middle_name: String::new(),
                family_name: "Example".to_string(),
                prefix: String::new(),
                suffix: String::new(),
                nickname: String::new(),
                phonetic_given_name: String::new(),
                phonetic_family_name: String::new(),
            },
            phones: Vec::new(),
            emails: Vec::new(),
            addresses: Vec::new(),
            organization: ContactOrganizationValue {
                company: String::new(),
                department: String::new(),
                title: String::new(),
            },
            note: String::new(),
        }
    }
}
