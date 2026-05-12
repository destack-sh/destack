use std::collections::BTreeSet;

use vobject::write_component;

use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::request::linux::eds::{
    EdsSourceDescriptor, EdsSourceKind, composite_eds_identifier, list_eds_sources,
    parse_composite_eds_identifier,
};
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    CalendarAccess, CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue,
    CalendarEventValue,
};
use crate::runtime::action::ActionSet;

use super::draft::{calendar_component_from_draft, create_calendar_event, validate_event_draft};
use super::event::{
    append_calendar_events, apply_query_filters, calendar_event_from_component, event_list_query,
    first_calendar_event,
};
use super::provider::{
    get_calendar_component, get_calendar_components, modify_calendar_objects,
    remove_calendar_objects,
};
use super::{
    CALENDAR_EVENT_CREATE_OPERATION, CALENDAR_EVENT_DELETE_OPERATION,
    CALENDAR_EVENT_LIST_OPERATION, CALENDAR_EVENT_READ_OPERATION, CALENDAR_EVENT_UPDATE_OPERATION,
    CALENDAR_LIST_OPERATION,
};

/// Return dynamic Unix calendar actions for Linux hosts.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::new()
}

/// Submit one Linux calendar request through EDS.
pub(crate) fn submit_calendar_request(
    _context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // list calendars
        HostRequest::OsCalendarList => {
            let calendars = list_calendars()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarDescriptors(calendars),
            )))
        }

        // list events
        HostRequest::OsCalendarEventList { query } => {
            let events = list_events(query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvents(events),
            )))
        }

        // read one event
        HostRequest::OsCalendarEventRead { id } => {
            let event = read_event(id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvent(event),
            )))
        }

        // create one event
        HostRequest::OsCalendarEventCreate { event } => {
            let id = create_event(event)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }

        // update one event
        HostRequest::OsCalendarEventUpdate { id, event } => {
            update_event(id, event)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // delete one event
        HostRequest::OsCalendarEventDelete { id } => {
            delete_event(id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// List readable calendars from enabled EDS sources.
fn list_calendars() -> RuntimeResult<Vec<CalendarDescriptorValue>> {
    let sources = list_eds_sources(EdsSourceKind::Calendar, CALENDAR_LIST_OPERATION)?;
    let primary_calendar_id = sources
        .iter()
        .find(|source| source.is_writable)
        .map(|source| source.uid.clone())
        .or_else(|| sources.first().map(|source| source.uid.clone()));
    let mut calendars = Vec::with_capacity(sources.len());

    // descriptor materialization
    for source in sources {
        calendars.push(CalendarDescriptorValue {
            id: source.uid.clone(),
            title: source.title.clone(),
            source: "Evolution".to_string(),
            owner: None,
            color_argb: source.color_argb.unwrap_or(0),
            primary: primary_calendar_id
                .as_deref()
                .is_some_and(|id| id == source.uid),
            access: if source.is_writable {
                CalendarAccess::Write
            } else {
                CalendarAccess::Read
            },
        });
    }

    Ok(calendars)
}

/// List calendar events from readable EDS calendars.
fn list_events(query: &CalendarEventQueryValue) -> RuntimeResult<Vec<CalendarEventValue>> {
    let sources = calendar_sources_for_query(query, CALENDAR_EVENT_LIST_OPERATION)?;
    let provider_query = event_list_query(query);
    let mut events = Vec::new();

    // provider source scan
    for source in sources {
        let calendars =
            get_calendar_components(&source.uid, &provider_query, CALENDAR_EVENT_LIST_OPERATION)?;

        // calendar materialization
        for calendar in calendars {
            append_calendar_events(&mut events, &source.uid, &calendar, query)?;
        }
    }

    apply_query_filters(&mut events, query);

    Ok(events)
}

/// Read one calendar event by stable identifier.
fn read_event(id: &str) -> RuntimeResult<CalendarEventValue> {
    let (source_uid, event_uid) =
        parse_composite_eds_identifier(id, CALENDAR_EVENT_READ_OPERATION, "id")?;
    let calendar = get_calendar_component(&source_uid, &event_uid, CALENDAR_EVENT_READ_OPERATION)?;
    let event = first_calendar_event(&calendar, CALENDAR_EVENT_READ_OPERATION)?;

    calendar_event_from_component(&source_uid, event, true)
}

/// Create one calendar event in one writable EDS source.
fn create_event(draft: &CalendarEventDraftValue) -> RuntimeResult<String> {
    validate_event_draft(draft, CALENDAR_EVENT_CREATE_OPERATION)?;
    ensure_writable_calendar_source(&draft.calendar_id, CALENDAR_EVENT_CREATE_OPERATION)?;

    let created_uid = create_calendar_event(draft, CALENDAR_EVENT_CREATE_OPERATION)?;

    Ok(composite_eds_identifier(&draft.calendar_id, &created_uid))
}

/// Update one existing calendar event in place.
fn update_event(id: &str, draft: &CalendarEventDraftValue) -> RuntimeResult<()> {
    validate_event_draft(draft, CALENDAR_EVENT_UPDATE_OPERATION)?;
    let (source_uid, event_uid) =
        parse_composite_eds_identifier(id, CALENDAR_EVENT_UPDATE_OPERATION, "id")?;

    if !draft.calendar_id.is_empty() && draft.calendar_id != source_uid {
        return Err(io_operation_error(
            CALENDAR_EVENT_UPDATE_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            "unix calendar event updates cannot move one event across calendars",
        ));
    }

    ensure_writable_calendar_source(&source_uid, CALENDAR_EVENT_UPDATE_OPERATION)?;

    let draft = CalendarEventDraftValue {
        calendar_id: source_uid.clone(),
        ..draft.clone()
    };
    let calendar =
        calendar_component_from_draft(&draft, Some(&event_uid), CALENDAR_EVENT_UPDATE_OPERATION)?;

    modify_calendar_objects(
        &source_uid,
        &[write_component(&calendar)],
        CALENDAR_EVENT_UPDATE_OPERATION,
    )
}

/// Delete one existing calendar event.
fn delete_event(id: &str) -> RuntimeResult<()> {
    let (source_uid, event_uid) =
        parse_composite_eds_identifier(id, CALENDAR_EVENT_DELETE_OPERATION, "id")?;
    ensure_writable_calendar_source(&source_uid, CALENDAR_EVENT_DELETE_OPERATION)?;

    remove_calendar_objects(
        &source_uid,
        &[(event_uid, String::new())],
        CALENDAR_EVENT_DELETE_OPERATION,
    )
}

/// Return calendar sources selected by the query.
fn calendar_sources_for_query(
    query: &CalendarEventQueryValue,
    operation: &'static str,
) -> RuntimeResult<Vec<EdsSourceDescriptor>> {
    let sources = list_eds_sources(EdsSourceKind::Calendar, operation)?;
    if query.calendar_ids.is_empty() {
        return Ok(sources);
    }

    let requested_ids = query.calendar_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    let mut found_ids = BTreeSet::new();

    // explicit source filter
    for source in sources {
        if query.calendar_ids.iter().any(|id| id == &source.uid) {
            found_ids.insert(source.uid.clone());
            selected.push(source);
        }
    }

    // missing source ids
    for calendar_id in requested_ids {
        if found_ids.contains(&calendar_id) {
            continue;
        }

        return Err(io_not_found(
            operation,
            format!("evolution calendar source `{calendar_id}` was not found"),
        ));
    }

    Ok(selected)
}

/// Ensure one calendar source remains writable.
fn ensure_writable_calendar_source(source_uid: &str, operation: &'static str) -> RuntimeResult<()> {
    let sources = list_eds_sources(EdsSourceKind::Calendar, operation)?;
    let source = sources
        .into_iter()
        .find(|source| source.uid == source_uid)
        .ok_or_else(|| io_not_found(operation, "evolution calendar source was not found"))?;

    if source.is_writable {
        return Ok(());
    }

    Err(io_operation_error(
        operation,
        Some(PlatformErrorCode::IoPermissionDenied),
        "evolution calendar source is not writable",
    ))
}
