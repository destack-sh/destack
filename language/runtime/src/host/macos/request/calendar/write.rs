use objc2_event_kit::{EKEvent, EKSpan};
use objc2_foundation::{NSArray, NSString};

use super::core::{
    CALENDAR_EVENT_CREATE_OPERATION, CALENDAR_EVENT_DELETE_OPERATION,
    CALENDAR_EVENT_UPDATE_OPERATION, calendar_error, calendar_invalid_argument, new_event_store,
    ns_date_from_unix_ns, ns_time_zone_from_name, ns_url_from_string,
};
use super::query::{resolve_calendar, resolve_event};
use super::recurrence::{
    calendar_availability_to_native, recurrence_rule_to_native, reminder_to_native,
};
use crate::diagnostic::RuntimeResult;
use crate::host::apple::execution::call_process_main_context_if_needed;
use crate::platform::os::abi_generated::CalendarEventDraftValue;

/// Create one EventKit event and return its saved identifier.
pub(super) fn create_event(draft: &CalendarEventDraftValue) -> RuntimeResult<String> {
    let draft = draft.clone();

    call_process_main_context_if_needed(move || {
        let store = new_event_store();
        let event = unsafe { EKEvent::eventWithEventStore(store.as_ref()) };

        // apply the runtime draft before saving
        apply_event_draft(store.as_ref(), event.as_ref(), &draft, false)?;

        // save one new event occurrence
        unsafe { store.saveEvent_span_error(event.as_ref(), EKSpan::ThisEvent) }.map_err(
            |error| {
                calendar_error(
                    CALENDAR_EVENT_CREATE_OPERATION,
                    format!("EventKit save failed: {error:?}"),
                )
            },
        )?;

        let id = unsafe { event.eventIdentifier() }.ok_or_else(|| {
            calendar_error(
                CALENDAR_EVENT_CREATE_OPERATION,
                "EventKit saved one event without one stable identifier",
            )
        })?;

        Ok(id.to_string())
    })
}

/// Update one existing EventKit event in place.
pub(super) fn update_event(id: &str, draft: &CalendarEventDraftValue) -> RuntimeResult<()> {
    let id = id.to_string();
    let draft = draft.clone();

    call_process_main_context_if_needed(move || {
        let store = new_event_store();
        let event = resolve_event(store.as_ref(), &id, CALENDAR_EVENT_UPDATE_OPERATION)?;

        // apply the runtime draft before saving
        apply_event_draft(store.as_ref(), event.as_ref(), &draft, true)?;

        // save one updated event occurrence
        unsafe { store.saveEvent_span_error(event.as_ref(), EKSpan::ThisEvent) }.map_err(
            |error| {
                calendar_error(
                    CALENDAR_EVENT_UPDATE_OPERATION,
                    format!("EventKit save failed: {error:?}"),
                )
            },
        )?;

        Ok(())
    })
}

/// Delete one existing EventKit event.
pub(super) fn delete_event(id: &str) -> RuntimeResult<()> {
    let id = id.to_string();

    call_process_main_context_if_needed(move || {
        let store = new_event_store();
        let event = resolve_event(store.as_ref(), &id, CALENDAR_EVENT_DELETE_OPERATION)?;

        // remove one resolved event occurrence
        unsafe { store.removeEvent_span_error(event.as_ref(), EKSpan::ThisEvent) }.map_err(
            |error| {
                calendar_error(
                    CALENDAR_EVENT_DELETE_OPERATION,
                    format!("EventKit delete failed: {error:?}"),
                )
            },
        )?;

        Ok(())
    })
}

/// Apply one runtime event draft to one native EventKit event.
fn apply_event_draft(
    store: &objc2_event_kit::EKEventStore,
    event: &EKEvent,
    draft: &CalendarEventDraftValue,
    is_update: bool,
) -> RuntimeResult<()> {
    validate_event_draft(draft, is_update)?;
    reject_unsupported_attendee_mutation(draft, is_update)?;

    let operation = draft_operation(is_update);
    let calendar = resolve_calendar(store, &draft.calendar_id, operation)?;
    let title = NSString::from_str(&draft.title);
    let start_date = ns_date_from_unix_ns(draft.start_unix_ns);
    let end_date = ns_date_from_unix_ns(draft.end_unix_ns);
    let notes = draft.notes.as_ref().map(|value| NSString::from_str(value));
    let location = draft
        .location
        .as_ref()
        .map(|value| NSString::from_str(value));
    let url = draft
        .url
        .as_ref()
        .map(|value| ns_url_from_string(value, operation))
        .transpose()?;
    let time_zone = draft
        .time_zone
        .as_ref()
        .map(|value| ns_time_zone_from_name(value, operation))
        .transpose()?;
    let availability = calendar_availability_to_native(draft.availability, operation)?;

    // core event fields
    unsafe {
        event.setCalendar(Some(calendar.as_ref()));
        event.setTitle(Some(&title));
        event.setStartDate(Some(start_date.as_ref()));
        event.setEndDate(Some(end_date.as_ref()));
        event.setAllDay(draft.all_day);
        event.setNotes(notes.as_deref());
        event.setLocation(location.as_deref());
        event.setURL(url.as_deref());
        event.setTimeZone(time_zone.as_deref());
        event.setAvailability(availability);
    }

    // recurrence writes are explicit only
    if let Some(rule) = &draft.recurrence_rule {
        let native_rule = recurrence_rule_to_native(rule, operation)?;
        let rules = NSArray::from_retained_slice(&[native_rule]);

        unsafe {
            event.setRecurrenceRules(Some(rules.as_ref()));
        }
    }

    // reminder writes are explicit only
    if let Some(reminders) = &draft.reminders {
        if reminders.is_empty() {
            unsafe {
                event.setAlarms(None);
            }
        } else {
            let native_reminders = reminders
                .iter()
                .map(|reminder| reminder_to_native(reminder, operation))
                .collect::<RuntimeResult<Vec<_>>>()?;
            let alarms = NSArray::from_retained_slice(&native_reminders);

            unsafe {
                event.setAlarms(Some(alarms.as_ref()));
            }
        }
    }

    Ok(())
}

/// Reject unsupported attendee writes on macOS EventKit.
fn reject_unsupported_attendee_mutation(
    draft: &CalendarEventDraftValue,
    is_update: bool,
) -> RuntimeResult<()> {
    if draft.attendees.is_none() {
        return Ok(());
    }

    Err(calendar_error(
        draft_operation(is_update),
        "macOS EventKit does not support mutating event attendees through this backend",
    ))
}

/// Validate one runtime event draft before native lowering.
fn validate_event_draft(draft: &CalendarEventDraftValue, is_update: bool) -> RuntimeResult<()> {
    let operation = draft_operation(is_update);

    // range sanity
    if draft.end_unix_ns < draft.start_unix_ns {
        return Err(calendar_invalid_argument(
            operation,
            "end_unix_ns must be greater than or equal to start_unix_ns",
        ));
    }

    // calendar target
    if draft.calendar_id.is_empty() {
        return Err(calendar_invalid_argument(
            operation,
            "calendar_id must not be empty",
        ));
    }

    Ok(())
}

/// Return the canonical write operation name.
fn draft_operation(is_update: bool) -> &'static str {
    if is_update {
        CALENDAR_EVENT_UPDATE_OPERATION
    } else {
        CALENDAR_EVENT_CREATE_OPERATION
    }
}
