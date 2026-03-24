use std::mem::MaybeUninit;
use std::sync::Arc;

use crate::host::abi::calendar::{
    HostCalendarAttendee, HostCalendarDescriptor, HostCalendarEvent, HostCalendarEventDraft,
    HostCalendarEventQuery, HostCalendarRecurrenceRule, HostCalendarRecurrenceWeekday,
    HostCalendarReminder,
};
use crate::host::abi::core::{HostOptionalStringRef, HostOptionalU32, HostOptionalU64};
use crate::host::core::registry::HostSessionRegistrationGuard;
use crate::host::core::{HostQueue, HostSessionRegistry};
use crate::host::ios::abi::bindings::{IosHostBindings, destack_host_ios_register_bindings};
use crate::host::ios::abi::calendar::callbacks::IosHostCalendarCallbacks;
use crate::host::ios::abi::calendar::ffi::{
    destack_host_ios_calendar_event_create, destack_host_ios_calendar_event_delete,
    destack_host_ios_calendar_event_list, destack_host_ios_calendar_event_read,
    destack_host_ios_calendar_event_update, destack_host_ios_calendar_list,
};
use crate::host::ios::abi::registry::unregister_ios_bindings;
use crate::host::{
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    Platform,
};
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
};
use crate::platform::os::{CalendarAccess, CalendarAvailability, CalendarRecurrenceFrequency};
use crate::platform::{NativeAbiCodec, NativeArray, NativeStringRef};

/// Register one temporary iOS host queue and keep registration state alive.
fn register_ios_runtime() -> (Arc<HostQueue>, HostSessionRegistrationGuard, u64) {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostSessionRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::clone(&queue),
        Some(unregister_ios_bindings),
    );
    let runtime_id = registration.host_session_id().0;

    (queue, registration, runtime_id)
}

/// Register one runtime-scoped iOS host bindings payload.
fn register_ios_bindings(runtime_id: u64, bindings: IosHostBindings) -> u32 {
    unsafe { destack_host_ios_register_bindings(runtime_id, bindings) }
}

unsafe extern "C" fn test_calendar_list_callback(
    _runtime_id: u64,
    output_calendars: *mut NativeArray<HostCalendarDescriptor>,
) -> u32 {
    unsafe {
        *output_calendars = test_host_calendar_descriptors();
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_calendar_event_list_callback(
    _runtime_id: u64,
    query: HostCalendarEventQuery,
    output_events: *mut NativeArray<HostCalendarEvent>,
) -> u32 {
    let query = unsafe { query.into_value() }.unwrap();
    assert_eq!(query, test_calendar_query());

    unsafe {
        *output_events = test_host_calendar_events();
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_calendar_event_read_callback(
    _runtime_id: u64,
    id: NativeStringRef,
    output_event: *mut HostCalendarEvent,
) -> u32 {
    let id = unsafe { id.as_str() }.unwrap();
    assert_eq!(id, "event-1");

    unsafe {
        *output_event = test_host_calendar_event();
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_calendar_event_create_callback(
    _runtime_id: u64,
    event: HostCalendarEventDraft,
    output_id: *mut NativeStringRef,
) -> u32 {
    let event = unsafe { event.into_value() }.unwrap();
    assert_eq!(event, test_calendar_event_draft());

    unsafe {
        *output_id = NativeStringRef::from("event-1");
    }

    HOST_STATUS_OK
}

unsafe extern "C" fn test_calendar_event_update_callback(
    _runtime_id: u64,
    id: NativeStringRef,
    event: HostCalendarEventDraft,
) -> u32 {
    let id = unsafe { id.as_str() }.unwrap();
    let event = unsafe { event.into_value() }.unwrap();

    assert_eq!(id, "event-1");
    assert_eq!(event, test_calendar_event_draft());

    HOST_STATUS_OK
}

unsafe extern "C" fn test_calendar_event_delete_callback(
    _runtime_id: u64,
    id: NativeStringRef,
) -> u32 {
    let id = unsafe { id.as_str() }.unwrap();
    assert_eq!(id, "event-1");

    HOST_STATUS_OK
}

#[test]
fn test_calendar_callbacks_report_missing_runtime_registration() {
    let runtime_id = HostSessionRegistry::allocate_session_id().0;
    let mut output = MaybeUninit::<NativeArray<HostCalendarDescriptor>>::uninit();
    let status = unsafe { destack_host_ios_calendar_list(runtime_id, output.as_mut_ptr()) };

    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_calendar_callbacks_report_unsupported_without_registered_handler() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(runtime_id, IosHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let mut output = MaybeUninit::<NativeArray<HostCalendarDescriptor>>::uninit();
    let status = unsafe { destack_host_ios_calendar_list(runtime_id, output.as_mut_ptr()) };
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_calendar_callbacks_route_registered_handlers() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            calendar: IosHostCalendarCallbacks {
                list: Some(test_calendar_list_callback),
                event_list: Some(test_calendar_event_list_callback),
                event_read: Some(test_calendar_event_read_callback),
                event_create: Some(test_calendar_event_create_callback),
                event_update: Some(test_calendar_event_update_callback),
                event_delete: Some(test_calendar_event_delete_callback),
            },
            ..IosHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let mut list_output = MaybeUninit::<NativeArray<HostCalendarDescriptor>>::uninit();
    let list_status =
        unsafe { destack_host_ios_calendar_list(runtime_id, list_output.as_mut_ptr()) };
    assert_eq!(list_status, HOST_STATUS_OK);
    let calendars = unsafe { list_output.assume_init().into_value() }.unwrap();
    assert_eq!(calendars, vec![test_calendar_descriptor()]);

    let mut event_list_output = MaybeUninit::<NativeArray<HostCalendarEvent>>::uninit();
    let event_list_status = unsafe {
        destack_host_ios_calendar_event_list(
            runtime_id,
            test_host_calendar_query(),
            event_list_output.as_mut_ptr(),
        )
    };
    assert_eq!(event_list_status, HOST_STATUS_OK);
    let events = unsafe { event_list_output.assume_init().into_value() }.unwrap();
    assert_eq!(events, vec![test_calendar_event()]);

    let mut event_read_output = MaybeUninit::<HostCalendarEvent>::uninit();
    let event_read_status = unsafe {
        destack_host_ios_calendar_event_read(
            runtime_id,
            NativeStringRef::from("event-1"),
            event_read_output.as_mut_ptr(),
        )
    };
    assert_eq!(event_read_status, HOST_STATUS_OK);
    let event = unsafe { event_read_output.assume_init().into_value() }.unwrap();
    assert_eq!(event, test_calendar_event());

    let mut create_output = MaybeUninit::<NativeStringRef>::uninit();
    let create_status = unsafe {
        destack_host_ios_calendar_event_create(
            runtime_id,
            test_host_calendar_event_draft(),
            create_output.as_mut_ptr(),
        )
    };
    assert_eq!(create_status, HOST_STATUS_OK);
    let create_output = unsafe { create_output.assume_init() };
    assert_eq!(unsafe { create_output.as_str() }.unwrap(), "event-1");

    let update_status = unsafe {
        destack_host_ios_calendar_event_update(
            runtime_id,
            NativeStringRef::from("event-1"),
            test_host_calendar_event_draft(),
        )
    };
    assert_eq!(update_status, HOST_STATUS_OK);

    let delete_status = unsafe {
        destack_host_ios_calendar_event_delete(runtime_id, NativeStringRef::from("event-1"))
    };
    assert_eq!(delete_status, HOST_STATUS_OK);
}

#[test]
fn test_calendar_callbacks_require_output_pointers() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            calendar: IosHostCalendarCallbacks {
                list: Some(test_calendar_list_callback),
                event_list: Some(test_calendar_event_list_callback),
                event_read: Some(test_calendar_event_read_callback),
                event_create: Some(test_calendar_event_create_callback),
                ..IosHostCalendarCallbacks::default()
            },
            ..IosHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let list_status = unsafe { destack_host_ios_calendar_list(runtime_id, std::ptr::null_mut()) };
    assert_eq!(list_status, HOST_STATUS_INVALID_ARGUMENT);

    let event_list_status = unsafe {
        destack_host_ios_calendar_event_list(
            runtime_id,
            test_host_calendar_query(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(event_list_status, HOST_STATUS_INVALID_ARGUMENT);

    let event_read_status = unsafe {
        destack_host_ios_calendar_event_read(
            runtime_id,
            NativeStringRef::from("event-1"),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(event_read_status, HOST_STATUS_INVALID_ARGUMENT);

    let event_create_status = unsafe {
        destack_host_ios_calendar_event_create(
            runtime_id,
            test_host_calendar_event_draft(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(event_create_status, HOST_STATUS_INVALID_ARGUMENT);
}

fn leak_array<T>(values: Vec<T>) -> NativeArray<T> {
    let values = values.into_boxed_slice();
    let len = values.len() as u32;
    let data = Box::leak(values).as_mut_ptr();

    NativeArray {
        data,
        len,
        capacity: len,
    }
}

fn test_host_calendar_descriptors() -> NativeArray<HostCalendarDescriptor> {
    leak_array(vec![test_host_calendar_descriptor()])
}

fn test_host_calendar_descriptor() -> HostCalendarDescriptor {
    HostCalendarDescriptor {
        id: NativeStringRef::from("calendar-1"),
        title: NativeStringRef::from("Personal"),
        source: NativeStringRef::from("local"),
        owner: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("user@example.com"),
        },
        color_argb: 0xff336699,
        primary: true,
        access: CalendarAccess::Write,
    }
}

fn test_calendar_descriptor() -> CalendarDescriptorValue {
    CalendarDescriptorValue {
        id: "calendar-1".to_string(),
        title: "Personal".to_string(),
        source: "local".to_string(),
        owner: Some("user@example.com".to_string()),
        color_argb: 0xff336699,
        primary: true,
        access: CalendarAccess::Write,
    }
}

fn test_host_calendar_query() -> HostCalendarEventQuery {
    HostCalendarEventQuery {
        calendar_ids: leak_array(vec![NativeStringRef::from("calendar-1")]),
        start_unix_ns: 1_000,
        end_unix_ns: 2_000,
        limit: HostOptionalU32 {
            has_value: true,
            value: 10,
        },
        include_canceled: false,
        include_declined: false,
        include_recurrence_instances: true,
    }
}

fn test_calendar_query() -> CalendarEventQueryValue {
    CalendarEventQueryValue {
        calendar_ids: vec!["calendar-1".to_string()],
        start_unix_ns: 1_000,
        end_unix_ns: 2_000,
        limit: Some(10),
        include_canceled: false,
        include_declined: false,
        include_recurrence_instances: true,
    }
}

fn test_host_calendar_events() -> NativeArray<HostCalendarEvent> {
    leak_array(vec![test_host_calendar_event()])
}

fn test_host_calendar_event() -> HostCalendarEvent {
    HostCalendarEvent {
        id: NativeStringRef::from("event-1"),
        calendar_id: NativeStringRef::from("calendar-1"),
        title: NativeStringRef::from("Standup"),
        notes: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("daily sync"),
        },
        location: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("Room 1"),
        },
        start_unix_ns: 1_200,
        end_unix_ns: 1_500,
        all_day: false,
        canceled: false,
        time_zone: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("UTC"),
        },
        availability: CalendarAvailability::Busy,
        url: HostOptionalStringRef::none(),
        organizer_name: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("Casey"),
        },
        organizer_email: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("casey@example.com"),
        },
        recurring: false,
        recurrence_master_id: HostOptionalStringRef::none(),
        recurrence_id_unix_ns: HostOptionalU64::none(),
        has_recurrence_rule: false,
        recurrence_rule: empty_recurrence_rule(),
        has_attendees: false,
        attendees: leak_array(Vec::<HostCalendarAttendee>::new()),
        has_reminders: false,
        reminders: leak_array(Vec::<HostCalendarReminder>::new()),
    }
}

fn test_calendar_event() -> CalendarEventValue {
    CalendarEventValue {
        id: "event-1".to_string(),
        calendar_id: "calendar-1".to_string(),
        title: "Standup".to_string(),
        notes: Some("daily sync".to_string()),
        location: Some("Room 1".to_string()),
        start_unix_ns: 1_200,
        end_unix_ns: 1_500,
        all_day: false,
        canceled: false,
        time_zone: Some("UTC".to_string()),
        availability: CalendarAvailability::Busy,
        url: None,
        organizer_name: Some("Casey".to_string()),
        organizer_email: Some("casey@example.com".to_string()),
        recurring: false,
        recurrence_master_id: None,
        recurrence_id_unix_ns: None,
        recurrence_rule: None,
        attendees: None,
        reminders: None,
    }
}

fn test_host_calendar_event_draft() -> HostCalendarEventDraft {
    HostCalendarEventDraft {
        calendar_id: NativeStringRef::from("calendar-1"),
        title: NativeStringRef::from("Standup"),
        notes: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("daily sync"),
        },
        location: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("Room 1"),
        },
        start_unix_ns: 1_200,
        end_unix_ns: 1_500,
        all_day: false,
        time_zone: HostOptionalStringRef {
            has_value: true,
            value: NativeStringRef::from("UTC"),
        },
        availability: CalendarAvailability::Busy,
        url: HostOptionalStringRef::none(),
        has_recurrence_rule: false,
        recurrence_rule: empty_recurrence_rule(),
        has_attendees: false,
        attendees: leak_array(Vec::<HostCalendarAttendee>::new()),
        has_reminders: false,
        reminders: leak_array(Vec::<HostCalendarReminder>::new()),
    }
}

fn test_calendar_event_draft() -> CalendarEventDraftValue {
    CalendarEventDraftValue {
        calendar_id: "calendar-1".to_string(),
        title: "Standup".to_string(),
        notes: Some("daily sync".to_string()),
        location: Some("Room 1".to_string()),
        start_unix_ns: 1_200,
        end_unix_ns: 1_500,
        all_day: false,
        time_zone: Some("UTC".to_string()),
        availability: CalendarAvailability::Busy,
        url: None,
        recurrence_rule: None,
        attendees: None,
        reminders: None,
    }
}

fn empty_recurrence_rule() -> HostCalendarRecurrenceRule {
    HostCalendarRecurrenceRule {
        frequency: CalendarRecurrenceFrequency::Daily,
        interval: 0,
        count: HostOptionalU32::none(),
        until_unix_ns: HostOptionalU64::none(),
        by_week_days: leak_array(Vec::<u8>::new()),
        by_weekday_ordinals: leak_array(Vec::<HostCalendarRecurrenceWeekday>::new()),
        by_month_days: leak_array(Vec::<i8>::new()),
        by_months: leak_array(Vec::<u8>::new()),
        by_year_days: leak_array(Vec::<i16>::new()),
        by_week_numbers: leak_array(Vec::<i8>::new()),
        by_set_positions: leak_array(Vec::<i16>::new()),
    }
}
