use crate::diagnostic::RuntimeResult;
use crate::host::core::error::invalid_argument_value;
use crate::platform::os::abi_generated::{ContactPageValue, ContactQueryValue, ContactValue};

/// Return one paged contact result for the requested cursor and limit.
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

/// Parse one opaque contact cursor into one stable offset.
fn parse_contact_cursor(cursor: Option<&str>) -> RuntimeResult<usize> {
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
