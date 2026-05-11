use std::sync::Arc;

use windows::ApplicationModel::Appointments::{Appointment, AppointmentStoreAccessType};
use windows::Foundation::{DateTime, TimeSpan};
use windows::core::Error as WindowsError;

use super::draft::apply_event_draft;
use super::event::calendar_event_from_native;
use super::store::{
    find_appointment_by_id, read_calendar_events, request_appointment_store,
    writable_calendar_for_id,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::error::invalid_argument_value;
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
};
use crate::runtime::service::executor::thread::ServiceThreadExecutor;
use crate::runtime::service::registry::global_service;
use crate::runtime::{ExecutionAffinity, ExecutionMode, ExecutionPolicy};

/// The Windows epoch offset from 1601 to 1970 in 100ns ticks.
pub(super) const WINDOWS_EPOCH_OFFSET_100NS: u64 = 116_444_736_000_000_000;

/// The runtime-owned Windows calendar name for writable fallback storage.
pub(super) const WINDOWS_APP_CALENDAR_NAME: &str = "Destack";

/// One process-global Windows calendar service.
pub(crate) struct WindowsCalendarService {
    /// Dedicated WinRT executor for calendar store work.
    executor: ServiceThreadExecutor<()>,
}

impl WindowsCalendarService {
    /// The execution policy for the Windows calendar service.
    pub(crate) const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Thread)
        .with_affinity(ExecutionAffinity::WindowsMta);

    /// List calendars through the Windows appointment store.
    pub(crate) fn list_calendars(
        &self,
        operation: &'static str,
    ) -> RuntimeResult<Vec<CalendarDescriptorValue>> {
        self.executor.call(operation, move |_state| {
            let store = request_appointment_store(
                AppointmentStoreAccessType::AllCalendarsReadOnly,
                operation,
            )?;
            let calendars = super::store::list_native_calendars(&store, operation)?;

            super::store::native_calendar_descriptors(calendars, operation)
        })
    }

    /// List events through the Windows appointment store.
    pub(crate) fn list_events(
        &self,
        query: &CalendarEventQueryValue,
        operation: &'static str,
    ) -> RuntimeResult<Vec<CalendarEventValue>> {
        let query = query.clone();

        self.executor.call(operation, move |_state| {
            let store = request_appointment_store(
                AppointmentStoreAccessType::AllCalendarsReadOnly,
                operation,
            )?;
            let calendars =
                super::store::calendars_for_query(&store, &query.calendar_ids, operation)?;
            let mut events = Vec::new();

            // collect one calendar slice at a time
            for calendar in &calendars {
                let mut calendar_events = read_calendar_events(calendar, &query, operation)?;
                events.append(&mut calendar_events);
            }

            // normalize order and apply one final global limit
            events.sort_by_key(|event| (event.start_unix_ns, event.id.clone()));

            if let Some(limit) = query.limit {
                events.truncate(limit as usize);
            }

            Ok(events)
        })
    }

    /// Read one event by identifier through the Windows appointment store.
    pub(crate) fn read_event(
        &self,
        id: &str,
        operation: &'static str,
    ) -> RuntimeResult<CalendarEventValue> {
        let id = id.to_string();

        self.executor.call(operation, move |_state| {
            let store = request_appointment_store(
                AppointmentStoreAccessType::AllCalendarsReadOnly,
                operation,
            )?;
            let (_calendar, appointment) = find_appointment_by_id(&store, &id, operation)?;

            calendar_event_from_native(&appointment, operation)
        })
    }

    /// Create one event through the writable Windows appointment store.
    pub(crate) fn create_event(
        &self,
        draft: &CalendarEventDraftValue,
        operation: &'static str,
    ) -> RuntimeResult<String> {
        let draft = draft.clone();

        self.executor.call(operation, move |_state| {
            let store = request_appointment_store(
                AppointmentStoreAccessType::AppCalendarsReadWrite,
                operation,
            )?;
            let calendar = writable_calendar_for_id(&store, &draft.calendar_id, operation)?;
            let appointment = Appointment::new()
                .map_err(|error| windows_calendar_error(operation, "Appointment::new", &error))?;

            // apply the runtime draft before saving
            apply_event_draft(&appointment, &draft, operation)?;

            calendar
                .SaveAppointmentAsync(&appointment)
                .map_err(|error| {
                    windows_calendar_error(
                        operation,
                        "AppointmentCalendar::SaveAppointmentAsync",
                        &error,
                    )
                })?
                .get()
                .map_err(|error| windows_calendar_error(operation, "IAsyncAction::get", &error))?;

            let id = appointment
                .LocalId()
                .map_err(|error| windows_calendar_error(operation, "Appointment::LocalId", &error))?
                .to_string();

            if id.is_empty() {
                return Err(io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    "windows calendar created one event without one stable identifier",
                ));
            }

            Ok(id)
        })
    }

    /// Update one existing writable event through the Windows appointment store.
    pub(crate) fn update_event(
        &self,
        id: &str,
        draft: &CalendarEventDraftValue,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        let id = id.to_string();
        let draft = draft.clone();

        self.executor.call(operation, move |_state| {
            let store = request_appointment_store(
                AppointmentStoreAccessType::AppCalendarsReadWrite,
                operation,
            )?;
            let (calendar, appointment) = find_appointment_by_id(&store, &id, operation)?;
            let calendar_id = appointment
                .CalendarId()
                .map_err(|error| {
                    windows_calendar_error(operation, "Appointment::CalendarId", &error)
                })?
                .to_string();

            // keep one update on the owning app calendar
            if !draft.calendar_id.is_empty() && draft.calendar_id != calendar_id {
                return Err(io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    "windows calendar event updates cannot move one event across calendars",
                ));
            }

            // apply the runtime draft before saving
            apply_event_draft(&appointment, &draft, operation)?;

            calendar
                .SaveAppointmentAsync(&appointment)
                .map_err(|error| {
                    windows_calendar_error(
                        operation,
                        "AppointmentCalendar::SaveAppointmentAsync",
                        &error,
                    )
                })?
                .get()
                .map_err(|error| windows_calendar_error(operation, "IAsyncAction::get", &error))?;

            Ok(())
        })
    }

    /// Delete one existing writable event through the Windows appointment store.
    pub(crate) fn delete_event(&self, id: &str, operation: &'static str) -> RuntimeResult<()> {
        let id = id.to_string();

        self.executor.call(operation, move |_state| {
            let store = request_appointment_store(
                AppointmentStoreAccessType::AppCalendarsReadWrite,
                operation,
            )?;
            let (calendar, appointment) = find_appointment_by_id(&store, &id, operation)?;
            let local_id = appointment.LocalId().map_err(|error| {
                windows_calendar_error(operation, "Appointment::LocalId", &error)
            })?;

            calendar
                .DeleteAppointmentAsync(&local_id)
                .map_err(|error| {
                    windows_calendar_error(
                        operation,
                        "AppointmentCalendar::DeleteAppointmentAsync",
                        &error,
                    )
                })?
                .get()
                .map_err(|error| windows_calendar_error(operation, "IAsyncAction::get", &error))?;

            Ok(())
        })
    }
}

/// Return the shared Windows calendar service.
pub(crate) fn windows_calendar_service(
    _operation: &'static str,
) -> RuntimeResult<Arc<WindowsCalendarService>> {
    global_service(|| {
        let executor = ServiceThreadExecutor::spawn(
            "destack-windows-calendar",
            WindowsCalendarService::POLICY,
            || Ok(()),
        )?;

        Ok(WindowsCalendarService { executor })
    })
}

/// Return one Windows DateTime for one UTC unix timestamp.
pub(super) fn datetime_from_unix_ns(unix_ns: u64) -> DateTime {
    let unix_100ns = unix_ns / 100;

    DateTime {
        UniversalTime: (WINDOWS_EPOCH_OFFSET_100NS + unix_100ns) as i64,
    }
}

/// Return one UTC unix timestamp for one Windows DateTime.
pub(super) fn unix_ns_from_datetime(date_time: DateTime) -> u64 {
    let ticks_100ns = date_time.UniversalTime.max(0) as u64;
    let unix_100ns = ticks_100ns.saturating_sub(WINDOWS_EPOCH_OFFSET_100NS);

    unix_100ns.saturating_mul(100)
}

/// Return one Windows TimeSpan for one nanosecond duration.
pub(super) fn timespan_from_ns(duration_ns: i64) -> TimeSpan {
    TimeSpan {
        Duration: duration_ns / 100,
    }
}

/// Return one nanosecond duration for one Windows TimeSpan.
pub(super) fn duration_ns_from_timespan(time_span: TimeSpan) -> u64 {
    time_span.Duration.max(0) as u64 * 100
}

/// Return one native duration between two unix timestamps.
pub(super) fn duration_timespan_from_bounds(
    start_unix_ns: u64,
    end_unix_ns: u64,
) -> RuntimeResult<TimeSpan> {
    if end_unix_ns <= start_unix_ns {
        return Err(invalid_argument_value(
            "event.end_unix_ns",
            "calendar event end timestamp must be greater than the start timestamp",
        ));
    }

    Ok(timespan_from_ns((end_unix_ns - start_unix_ns) as i64))
}

/// Return whether one WinRT error represents one missing entity.
pub(super) fn is_not_found_error(error: &WindowsError) -> bool {
    error.code().0 as u32 == 0x80070490
}

/// Return one optional runtime string, collapsing empty native strings.
pub(super) fn optional_hstring(value: windows::core::HSTRING) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    Some(value.to_string())
}

/// Map one WinRT calendar error into one runtime IO error.
pub(super) fn windows_calendar_error(
    operation: &'static str,
    stage: &str,
    error: &WindowsError,
) -> Box<RuntimeError> {
    match error.code().0 as u32 {
        0x80070490 => io_not_found(operation, "windows calendar entity was not found"),
        0x80070005 => io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            "windows calendar access was denied",
        ),
        _ => io_operation_error(operation, None, format!("{stage} failed: {error}")),
    }
}
