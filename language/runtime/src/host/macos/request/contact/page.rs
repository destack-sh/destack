use rustc_hash::FxHashSet;

use crate::platform::os::abi_generated::{ContactPageValue, ContactQueryValue, ContactValue};

/// Paginate one full contact result set.
pub(super) fn paginate_contacts(
    mut contacts: Vec<ContactValue>,
    query: &ContactQueryValue,
) -> ContactPageValue {
    sort_contacts(&mut contacts);

    let cursor = query
        .cursor
        .as_ref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_default();
    let limit = query
        .limit
        .map(|value| value as usize)
        .unwrap_or(contacts.len());
    let page_contacts = contacts
        .into_iter()
        .skip(cursor)
        .take(limit)
        .collect::<Vec<_>>();
    let next_offset = cursor + page_contacts.len();
    let has_more = next_offset < contacts_len_for_cursor(query, next_offset, &page_contacts);

    let next_cursor = if has_more {
        next_offset.to_string()
    } else {
        String::new()
    };

    ContactPageValue {
        contacts: page_contacts,
        next_cursor,
        has_more,
    }
}

/// Return the full result length for one page cursor calculation.
fn contacts_len_for_cursor(
    query: &ContactQueryValue,
    next_offset: usize,
    page_contacts: &[ContactValue],
) -> usize {
    let page_len = page_contacts.len();
    let requested_limit = query.limit.map(|value| value as usize).unwrap_or(page_len);

    if page_len < requested_limit {
        return next_offset;
    }

    next_offset + 1
}

/// Append unique contacts by stable identifier.
pub(super) fn append_unique_contacts(
    contacts: &mut Vec<ContactValue>,
    seen_ids: &mut FxHashSet<String>,
    values: Vec<ContactValue>,
) {
    for value in values {
        if seen_ids.insert(value.id.clone()) {
            contacts.push(value);
        }
    }
}

/// Sort contacts into one stable user-facing order.
pub(super) fn sort_contacts(contacts: &mut [ContactValue]) {
    contacts.sort_by(|left, right| {
        left.name
            .family_name
            .cmp(&right.name.family_name)
            .then_with(|| left.name.given_name.cmp(&right.name.given_name))
            .then_with(|| left.name.middle_name.cmp(&right.name.middle_name))
            .then_with(|| left.organization.company.cmp(&right.organization.company))
            .then_with(|| left.id.cmp(&right.id))
    });
}
