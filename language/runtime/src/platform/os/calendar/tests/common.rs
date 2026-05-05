use std::sync::{Mutex, OnceLock};

use destack_vm::{BindingContext, StringHandle};
use destack_workspace::{RuntimeAppPermission, RuntimeOptions};

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "linux")]
use crate::host::os::linux::tests::{
    LinuxCalendarHooks as DesktopCalendarHooks, set_linux_calendar_test_hooks,
};
#[cfg(target_os = "macos")]
use crate::host::os::macos::request::calendar::{
    MacosCalendarHooks as DesktopCalendarHooks, set_macos_calendar_test_hooks,
};
#[cfg(windows)]
use crate::host::os::windows::request::calendar::{
    WindowsCalendarHooks as DesktopCalendarHooks, set_windows_calendar_test_hooks,
};
use crate::platform::abi::NativeStringRef;
use crate::platform::os::abi_generated::{
    CalendarAbsoluteReminderValue, CalendarDescriptorValue, CalendarEventDraftValue,
    CalendarEventQueryValue, CalendarEventValue, CalendarRelativeReminderValue,
    CalendarReminderValue,
};
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_configured_harness_context};
use crate::platform::os::{
    CalendarAccess, CalendarAvailability, CalendarDescriptor, CalendarDescriptorVm, CalendarEvent,
    CalendarEventDraft, CalendarEventDraftVm, CalendarEventQuery, CalendarEventQueryVm,
    CalendarEventVm,
};
use crate::platform::{NativeAbiCodec, NativeArray, VmAbiCodec, VmArray};

/// Shared desktop calendar test state.
#[derive(Default)]
pub(super) struct DesktopCalendarTestState {
    /// Recorded list-event queries.
    pub(super) event_queries: Vec<CalendarEventQueryValue>,
    /// Recorded event reads.
    pub(super) read_ids: Vec<String>,
    /// Recorded created drafts.
    pub(super) created_events: Vec<CalendarEventDraftValue>,
    /// Recorded updates.
    pub(super) updated_events: Vec<(String, CalendarEventDraftValue)>,
    /// Recorded deletions.
    pub(super) deleted_ids: Vec<String>,
}

/// Return the shared desktop calendar test state slot.
pub(super) fn desktop_calendar_test_state() -> &'static Mutex<DesktopCalendarTestState> {
    static STATE: OnceLock<Mutex<DesktopCalendarTestState>> = OnceLock::new();

    STATE.get_or_init(|| Mutex::new(DesktopCalendarTestState::default()))
}

/// Install calendar permissions for desktop host tests.
pub(super) fn enable_calendar_declarations(options: &mut RuntimeOptions) {
    options
        .app
        .permissions
        .insert(RuntimeAppPermission::CalendarRead);
    options
        .app
        .permissions
        .insert(RuntimeAppPermission::CalendarWrite);
}

/// Install one scoped desktop calendar hook set.
pub(super) fn with_desktop_calendar_test_hooks<T>(callback: impl FnOnce() -> T) -> T {
    {
        let mut state = desktop_calendar_test_state()
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        *state = DesktopCalendarTestState::default();
    }

    let hooks = desktop_calendar_test_hooks();
    set_desktop_calendar_test_hooks(hooks);
    let guard = DesktopCalendarHookGuard;
    let result = callback();

    drop(guard);

    result
}

/// Run one desktop calendar harness pass with hooks and declarations installed.
pub(super) fn with_desktop_calendar_context<T>(
    mut callback: impl for<'call> FnMut(HarnessContext<'call>) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut result = None;

    with_desktop_calendar_test_hooks(|| {
        with_configured_harness_context(enable_calendar_declarations, |context| {
            result = Some(callback(context));
            Ok(())
        });
    });

    result.expect("desktop calendar harness should capture one result")
}

/// Return the desktop calendar hook set for the active target.
fn desktop_calendar_test_hooks() -> DesktopCalendarHooks {
    DesktopCalendarHooks {
        list: Some(test_list_calendars),
        event_list: Some(test_list_events),
        event_read: Some(test_read_event),
        event_create: Some(test_create_event),
        event_update: Some(test_update_event),
        event_delete: Some(test_delete_event),
    }
}

/// Install the active target calendar hook set.
fn set_desktop_calendar_test_hooks(hooks: DesktopCalendarHooks) {
    #[cfg(target_os = "linux")]
    set_linux_calendar_test_hooks(hooks);

    #[cfg(target_os = "macos")]
    set_macos_calendar_test_hooks(hooks);

    #[cfg(windows)]
    set_windows_calendar_test_hooks(hooks);
}

/// One scoped desktop calendar hook installation.
struct DesktopCalendarHookGuard;

impl Drop for DesktopCalendarHookGuard {
    /// Clear the active desktop calendar hooks after one test.
    fn drop(&mut self) {
        set_desktop_calendar_test_hooks(DesktopCalendarHooks::default());
    }
}

/// Return one deterministic desktop calendar list.
fn test_list_calendars() -> RuntimeResult<Vec<CalendarDescriptorValue>> {
    Ok(vec![sample_calendar_descriptor()])
}

/// Return one deterministic desktop event list and record the query.
fn test_list_events(query: CalendarEventQueryValue) -> RuntimeResult<Vec<CalendarEventValue>> {
    let mut state = desktop_calendar_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.event_queries.push(query);

    Ok(vec![sample_calendar_event()])
}

/// Return one deterministic desktop event and record the read id.
fn test_read_event(id: String) -> RuntimeResult<CalendarEventValue> {
    let mut state = desktop_calendar_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.read_ids.push(id);

    Ok(sample_calendar_event())
}

/// Return one deterministic created event id and record the draft.
fn test_create_event(event: CalendarEventDraftValue) -> RuntimeResult<String> {
    let mut state = desktop_calendar_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.created_events.push(event);

    Ok("event-created".to_string())
}

/// Record one update request.
fn test_update_event(id: String, event: CalendarEventDraftValue) -> RuntimeResult<()> {
    let mut state = desktop_calendar_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.updated_events.push((id, event));

    Ok(())
}

/// Record one delete request.
fn test_delete_event(id: String) -> RuntimeResult<()> {
    let mut state = desktop_calendar_test_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    state.deleted_ids.push(id);

    Ok(())
}

/// Build one deterministic calendar descriptor fixture.
pub(super) fn sample_calendar_descriptor() -> CalendarDescriptorValue {
    CalendarDescriptorValue {
        id: "primary".to_string(),
        title: "Personal".to_string(),
        source: "Local".to_string(),
        owner: Some("Ada".to_string()),
        color_argb: 0xFF336699,
        primary: true,
        access: CalendarAccess::Write,
    }
}

/// Build one deterministic calendar event fixture.
pub(super) fn sample_calendar_event() -> CalendarEventValue {
    CalendarEventValue {
        id: "event-1".to_string(),
        calendar_id: "primary".to_string(),
        title: "Planning".to_string(),
        notes: Some("Weekly planning".to_string()),
        location: Some("Studio".to_string()),
        start_unix_ns: 1_700_000_000_000_000_000,
        end_unix_ns: 1_700_000_900_000_000_000,
        all_day: false,
        canceled: false,
        time_zone: Some("Europe/Zurich".to_string()),
        availability: CalendarAvailability::Busy,
        url: Some("https://example.com/event".to_string()),
        organizer_name: Some("Ada".to_string()),
        organizer_email: Some("ada@example.com".to_string()),
        recurring: false,
        recurrence_master_id: None,
        recurrence_id_unix_ns: None,
        recurrence_rule: None,
        attendees: None,
        reminders: Some(sample_calendar_reminders()),
    }
}

/// Build one deterministic calendar draft fixture.
pub(super) fn sample_calendar_draft(title: &str) -> CalendarEventDraftValue {
    CalendarEventDraftValue {
        calendar_id: "primary".to_string(),
        title: title.to_string(),
        notes: Some("Draft notes".to_string()),
        location: Some("Desk".to_string()),
        start_unix_ns: 1_700_000_000_000_000_000,
        end_unix_ns: 1_700_000_900_000_000_000,
        all_day: false,
        time_zone: Some("Europe/Zurich".to_string()),
        availability: CalendarAvailability::Busy,
        url: Some("https://example.com/draft".to_string()),
        recurrence_rule: None,
        attendees: None,
        reminders: Some(sample_calendar_reminders()),
    }
}

/// Build one deterministic calendar reminder fixture.
fn sample_calendar_reminders() -> Vec<CalendarReminderValue> {
    vec![
        CalendarReminderValue::CalendarRelativeReminder(CalendarRelativeReminderValue {
            kind: "relative".to_string(),
            minutes_before_start: 15,
        }),
        CalendarReminderValue::CalendarAbsoluteReminder(CalendarAbsoluteReminderValue {
            kind: "absolute".to_string(),
            absolute_unix_ns: 1_699_999_900_000_000_000,
        }),
    ]
}

/// Build one deterministic calendar query fixture.
pub(super) fn sample_calendar_query() -> CalendarEventQueryValue {
    CalendarEventQueryValue {
        calendar_ids: vec!["primary".to_string()],
        start_unix_ns: 1_699_999_000_000_000_000,
        end_unix_ns: 1_700_001_000_000_000_000,
        limit: Some(8),
        include_canceled: false,
        include_declined: false,
        include_recurrence_instances: true,
    }
}

/// Build one calendar query payload for the active harness.
pub(super) fn calendar_query_harness_value(
    context: &mut HarnessContext<'_>,
    query: CalendarEventQueryValue,
) -> RuntimeResult<HarnessValue<CalendarEventQuery, CalendarEventQueryVm>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let query = CalendarEventQueryVm::from_value(&mut vm_context.write(), query)?;

            Ok(HarnessValue::Vm(query))
        }
        None => Ok(HarnessValue::Native(CalendarEventQuery::from_value(
            context.call_context,
            query,
        ))),
    }
}

/// Build one calendar draft payload for the active harness.
pub(super) fn calendar_draft_harness_value(
    context: &mut HarnessContext<'_>,
    draft: CalendarEventDraftValue,
) -> HarnessValue<CalendarEventDraft, CalendarEventDraftVm> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let draft = CalendarEventDraftVm::from_value(&mut vm_context.write(), draft)
                .expect("vm calendar draft should encode");

            HarnessValue::Vm(draft)
        }
        None => HarnessValue::Native(CalendarEventDraft::from_value(context.call_context, draft)),
    }
}

/// Build one string payload for the active harness.
pub(super) fn string_harness_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let value = StringHandle::new(
                vm_context
                    .intern_string(value)
                    .expect("vm calendar test string should intern"),
            );

            HarnessValue::Vm(value)
        }
        None => HarnessValue::Native(context.call_context.store_string(value)),
    }
}

/// Decode one string payload into one owned string.
pub(super) fn decode_string_value(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<NativeStringRef, StringHandle>,
) -> RuntimeResult<String> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_str() }?.to_string()),
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut BindingContext<'_>)
            };

            vm_context
                .string_value(value.value())
                .map_err(|error| RuntimeError::from(error).boxed())
        }
    }
}

/// Decode one calendar descriptor list into owned values.
pub(super) fn decode_calendar_descriptors(
    context: &mut HarnessContext<'_>,
    descriptors: HarnessValue<NativeArray<CalendarDescriptor>, VmArray<CalendarDescriptorVm>>,
) -> RuntimeResult<Vec<CalendarDescriptorValue>> {
    match descriptors {
        HarnessValue::Native(descriptors) => {
            let descriptors = unsafe { descriptors.as_slice()? };
            let mut decoded = Vec::with_capacity(descriptors.len());

            for descriptor in descriptors {
                decoded.push(unsafe { CalendarDescriptor::into_value(*descriptor)? });
            }

            Ok(decoded)
        }
        HarnessValue::Vm(descriptors) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut BindingContext<'_>)
            };
            let descriptors = descriptors.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(descriptors.len());

            for descriptor in descriptors {
                decoded.push(CalendarDescriptorVm::into_value(
                    descriptor,
                    &vm_context.read(),
                )?);
            }

            Ok(decoded)
        }
    }
}

/// Decode one calendar event list into owned values.
pub(super) fn decode_calendar_events(
    context: &mut HarnessContext<'_>,
    events: HarnessValue<NativeArray<CalendarEvent>, VmArray<CalendarEventVm>>,
) -> RuntimeResult<Vec<CalendarEventValue>> {
    match events {
        HarnessValue::Native(events) => {
            let events = unsafe { events.as_slice()? };
            let mut decoded = Vec::with_capacity(events.len());

            for event in events {
                decoded.push(unsafe { CalendarEvent::into_value(*event)? });
            }

            Ok(decoded)
        }
        HarnessValue::Vm(events) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut BindingContext<'_>)
            };
            let events = events.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(events.len());

            for event in events {
                decoded.push(CalendarEventVm::into_value(event, &vm_context.read())?);
            }

            Ok(decoded)
        }
    }
}

/// Decode one calendar event into one owned value.
pub(super) fn decode_calendar_event(
    context: &mut HarnessContext<'_>,
    event: HarnessValue<CalendarEvent, CalendarEventVm>,
) -> RuntimeResult<CalendarEventValue> {
    match event {
        HarnessValue::Native(event) => unsafe { CalendarEvent::into_value(event) },
        HarnessValue::Vm(event) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut BindingContext<'_>)
            };

            CalendarEventVm::into_value(event, &vm_context.read())
        }
    }
}
