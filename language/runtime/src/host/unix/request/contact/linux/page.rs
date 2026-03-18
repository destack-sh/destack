use std::cmp::Ordering;

use crate::diagnostic::RuntimeResult;
use crate::host::core::error::invalid_argument_value;
use crate::platform::os::abi_generated::{
    ContactNameValue, ContactOrganizationValue, ContactPageValue, ContactQueryValue, ContactValue,
};

/// Return one stable cursor page for one contact result set.
pub(super) fn paginate_contacts(
    contacts: Vec<ContactValue>,
    query: &ContactQueryValue,
) -> RuntimeResult<ContactPageValue> {
    let cursor = parse_contact_cursor(query.cursor.as_deref())?;
    let limit = query.limit.unwrap_or(u32::MAX) as usize;
    let total = contacts.len();
    let start = cursor.min(total);
    let end = start.saturating_add(limit).min(total);
    let has_more = end < total;
    let next_cursor = if has_more {
        end.to_string()
    } else {
        String::new()
    };
    let contacts = contacts
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();

    Ok(ContactPageValue {
        contacts,
        next_cursor,
        has_more,
    })
}

/// Parse one opaque contact cursor into one decimal offset.
pub(super) fn parse_contact_cursor(cursor: Option<&str>) -> RuntimeResult<usize> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };

    if cursor.is_empty() {
        return Ok(0);
    }

    cursor.parse::<usize>().map_err(|_| {
        invalid_argument_value(
            "cursor",
            "contact cursor must contain one unsigned decimal offset",
        )
    })
}

/// Compare two contacts with one stable display-first ordering.
pub(super) fn compare_contacts(left: &ContactValue, right: &ContactValue) -> Ordering {
    let left_name = display_contact_name(&left.name, &left.organization).to_ascii_lowercase();
    let right_name = display_contact_name(&right.name, &right.organization).to_ascii_lowercase();

    left_name
        .cmp(&right_name)
        .then_with(|| left.id.cmp(&right.id))
}

/// Return one display name for one draft payload.
pub(super) fn display_contact_name(
    name: &ContactNameValue,
    organization: &ContactOrganizationValue,
) -> String {
    let mut parts = Vec::new();

    if !name.prefix.is_empty() {
        parts.push(name.prefix.as_str());
    }

    if !name.given_name.is_empty() {
        parts.push(name.given_name.as_str());
    }

    if !name.middle_name.is_empty() {
        parts.push(name.middle_name.as_str());
    }

    if !name.family_name.is_empty() {
        parts.push(name.family_name.as_str());
    }

    if !name.suffix.is_empty() {
        parts.push(name.suffix.as_str());
    }

    if !parts.is_empty() {
        return parts.join(" ");
    }

    if !organization.company.is_empty() {
        return organization.company.clone();
    }

    if !name.nickname.is_empty() {
        return name.nickname.clone();
    }

    "Contact".to_string()
}
