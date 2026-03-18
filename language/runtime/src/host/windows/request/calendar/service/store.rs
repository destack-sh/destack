use windows::ApplicationModel::Appointments::{
    Appointment, AppointmentCalendar, AppointmentCalendarOtherAppWriteAccess, AppointmentStore,
    AppointmentStoreAccessType, FindAppointmentsOptions,
};
use windows::core::HSTRING;

use super::core::{
    WINDOWS_APP_CALENDAR_NAME, datetime_from_unix_ns, is_not_found_error, windows_calendar_error,
};
use super::event::{
    calendar_event_from_native, event_overlaps_query, is_declined_for_current_user,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::core::io_not_found;
use crate::platform::os::abi_generated::{
    CalendarAccess, CalendarDescriptorValue, CalendarEventQueryValue, CalendarEventValue,
};

/// Return one appointment store with the requested access level.
pub(super) fn request_appointment_store(
    access_type: AppointmentStoreAccessType,
    operation: &'static str,
) -> RuntimeResult<AppointmentStore> {
    windows::ApplicationModel::Appointments::AppointmentManager::RequestStoreAsync(access_type)
        .map_err(|error| {
            windows_calendar_error(operation, "AppointmentManager::RequestStoreAsync", &error)
        })?
        .get()
        .map_err(|error| windows_calendar_error(operation, "IAsyncOperation::get", &error))
}

/// Return all calendars visible through one appointment store.
pub(super) fn list_native_calendars(
    store: &AppointmentStore,
    operation: &'static str,
) -> RuntimeResult<Vec<AppointmentCalendar>> {
    let calendars = store
        .FindAppointmentCalendarsAsync()
        .map_err(|error| {
            windows_calendar_error(
                operation,
                "AppointmentStore::FindAppointmentCalendarsAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| windows_calendar_error(operation, "IAsyncOperation::get", &error))?;
    let count = calendars
        .Size()
        .map_err(|error| windows_calendar_error(operation, "IVectorView::Size", &error))?;
    let mut values = Vec::with_capacity(count as usize);

    // decode one runtime calendar per native row
    for index in 0..count {
        let calendar = calendars
            .GetAt(index)
            .map_err(|error| windows_calendar_error(operation, "IVectorView::GetAt", &error))?;
        values.push(calendar);
    }

    Ok(values)
}

/// Return runtime calendar descriptors for one native calendar list.
pub(super) fn native_calendar_descriptors(
    calendars: Vec<AppointmentCalendar>,
    operation: &'static str,
) -> RuntimeResult<Vec<CalendarDescriptorValue>> {
    let mut writable_ranked = Vec::new();
    let mut descriptors = Vec::with_capacity(calendars.len());

    // decode one descriptor per calendar and keep one primary fallback candidate
    for calendar in &calendars {
        let write_access = calendar.OtherAppWriteAccess().map_err(|error| {
            windows_calendar_error(
                operation,
                "AppointmentCalendar::OtherAppWriteAccess",
                &error,
            )
        })?;
        let local_id = calendar
            .LocalId()
            .map_err(|error| {
                windows_calendar_error(operation, "AppointmentCalendar::LocalId", &error)
            })?
            .to_string();
        let access = if write_access != AppointmentCalendarOtherAppWriteAccess::None {
            writable_ranked.push(local_id.clone());
            CalendarAccess::Write
        } else {
            CalendarAccess::Read
        };

        descriptors.push(CalendarDescriptorValue {
            id: local_id,
            title: calendar
                .DisplayName()
                .map_err(|error| {
                    windows_calendar_error(operation, "AppointmentCalendar::DisplayName", &error)
                })?
                .to_string(),
            source: calendar
                .SourceDisplayName()
                .map_err(|error| {
                    windows_calendar_error(
                        operation,
                        "AppointmentCalendar::SourceDisplayName",
                        &error,
                    )
                })?
                .to_string(),
            owner: None,
            color_argb: 0,
            primary: false,
            access,
        });
    }

    let primary_id = writable_ranked
        .into_iter()
        .next()
        .or_else(|| descriptors.first().map(|value| value.id.clone()));

    // choose one best-effort default calendar marker
    if let Some(primary_id) = primary_id {
        for descriptor in &mut descriptors {
            descriptor.primary = descriptor.id == primary_id;
        }
    }

    Ok(descriptors)
}

/// Return the calendars selected by one runtime query.
pub(super) fn calendars_for_query(
    store: &AppointmentStore,
    calendar_ids: &[String],
    operation: &'static str,
) -> RuntimeResult<Vec<AppointmentCalendar>> {
    let all_calendars = list_native_calendars(store, operation)?;

    if calendar_ids.is_empty() {
        return Ok(all_calendars);
    }

    let mut calendars = Vec::with_capacity(calendar_ids.len());

    // resolve one explicit calendar per requested id
    for calendar_id in calendar_ids {
        let Some(calendar) = all_calendars.iter().find(|calendar| {
            calendar
                .LocalId()
                .map(|value| value == calendar_id.as_str())
                .unwrap_or(false)
        }) else {
            return Err(io_not_found(
                operation,
                format!("windows calendar `{calendar_id}` was not found"),
            ));
        };

        calendars.push(calendar.clone());
    }

    Ok(calendars)
}

/// Return one writable calendar for one explicit identifier or the default app calendar.
pub(super) fn writable_calendar_for_id(
    store: &AppointmentStore,
    calendar_id: &str,
    operation: &'static str,
) -> RuntimeResult<AppointmentCalendar> {
    if calendar_id.is_empty() {
        return default_app_calendar(store, operation);
    }

    let calendar_id = HSTRING::from(calendar_id);

    store
        .GetAppointmentCalendarAsync(&calendar_id)
        .map_err(|error| {
            windows_calendar_error(
                operation,
                "AppointmentStore::GetAppointmentCalendarAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| windows_calendar_error(operation, "IAsyncOperation::get", &error))
}

/// Return one writable app calendar, creating one when none exist yet.
pub(super) fn default_app_calendar(
    store: &AppointmentStore,
    operation: &'static str,
) -> RuntimeResult<AppointmentCalendar> {
    let calendars = list_native_calendars(store, operation)?;

    // prefer one existing writable app calendar
    for calendar in calendars {
        let write_access = calendar.OtherAppWriteAccess().map_err(|error| {
            windows_calendar_error(
                operation,
                "AppointmentCalendar::OtherAppWriteAccess",
                &error,
            )
        })?;

        if write_access != AppointmentCalendarOtherAppWriteAccess::None {
            return Ok(calendar);
        }
    }

    store
        .CreateAppointmentCalendarAsync(&HSTRING::from(WINDOWS_APP_CALENDAR_NAME))
        .map_err(|error| {
            windows_calendar_error(
                operation,
                "AppointmentStore::CreateAppointmentCalendarAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| windows_calendar_error(operation, "IAsyncOperation::get", &error))
}

/// Return one event and its calendar by scanning the visible calendars.
pub(super) fn find_appointment_by_id(
    store: &AppointmentStore,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<(AppointmentCalendar, Appointment)> {
    let id = HSTRING::from(id);
    let calendars = list_native_calendars(store, operation)?;

    // probe one calendar at a time for the requested local id
    for calendar in calendars {
        let appointment = match calendar.GetAppointmentAsync(&id) {
            Ok(operation_handle) => match operation_handle.get() {
                Ok(value) => value,
                Err(error) if is_not_found_error(&error) => continue,
                Err(error) => {
                    return Err(windows_calendar_error(
                        operation,
                        "IAsyncOperation::get",
                        &error,
                    ));
                }
            },
            Err(error) if is_not_found_error(&error) => continue,
            Err(error) => {
                return Err(windows_calendar_error(
                    operation,
                    "AppointmentCalendar::GetAppointmentAsync",
                    &error,
                ));
            }
        };

        return Ok((calendar, appointment));
    }

    Err(io_not_found(
        operation,
        "windows calendar event was not found",
    ))
}

/// Read one calendar slice for the requested query.
pub(super) fn read_calendar_events(
    calendar: &AppointmentCalendar,
    query: &CalendarEventQueryValue,
    operation: &'static str,
) -> RuntimeResult<Vec<CalendarEventValue>> {
    let range_start = datetime_from_unix_ns(query.start_unix_ns);
    let range_length = super::event::range_length_for_query(query)?;
    let options = find_appointments_options(query, operation)?;
    let native_events = if query.include_recurrence_instances {
        calendar
            .FindAppointmentsAsyncWithOptions(range_start, range_length, &options)
            .map_err(|error| {
                windows_calendar_error(
                    operation,
                    "AppointmentCalendar::FindAppointmentsAsyncWithOptions",
                    &error,
                )
            })?
            .get()
            .map_err(|error| windows_calendar_error(operation, "IAsyncOperation::get", &error))?
    } else {
        calendar
            .FindUnexpandedAppointmentsAsyncWithOptions(&options)
            .map_err(|error| {
                windows_calendar_error(
                    operation,
                    "AppointmentCalendar::FindUnexpandedAppointmentsAsyncWithOptions",
                    &error,
                )
            })?
            .get()
            .map_err(|error| windows_calendar_error(operation, "IAsyncOperation::get", &error))?
    };
    let count = native_events
        .Size()
        .map_err(|error| windows_calendar_error(operation, "IVectorView::Size", &error))?;
    let mut events = Vec::with_capacity(count as usize);

    // decode one runtime event per native result
    for index in 0..count {
        let appointment = native_events
            .GetAt(index)
            .map_err(|error| windows_calendar_error(operation, "IVectorView::GetAt", &error))?;
        let event = calendar_event_from_native(&appointment, operation)?;

        if !query.include_recurrence_instances
            && !event_overlaps_query(&event, query.start_unix_ns, query.end_unix_ns)
        {
            continue;
        }

        if !query.include_canceled && event.canceled {
            continue;
        }

        if !query.include_declined && is_declined_for_current_user(&event) {
            continue;
        }

        events.push(event);
    }

    Ok(events)
}

/// Build one native find-options payload for the requested query.
fn find_appointments_options(
    query: &CalendarEventQueryValue,
    operation: &'static str,
) -> RuntimeResult<FindAppointmentsOptions> {
    let options = FindAppointmentsOptions::new().map_err(|error| {
        windows_calendar_error(operation, "FindAppointmentsOptions::new", &error)
    })?;

    // keep hidden calendars available when the caller selected them explicitly
    options.SetIncludeHidden(true).map_err(|error| {
        windows_calendar_error(
            operation,
            "FindAppointmentsOptions::SetIncludeHidden",
            &error,
        )
    })?;

    // let the store trim one per-calendar list when the query already bounds it
    if let Some(limit) = query.limit {
        options.SetMaxCount(limit).map_err(|error| {
            windows_calendar_error(operation, "FindAppointmentsOptions::SetMaxCount", &error)
        })?;
    }

    Ok(options)
}
