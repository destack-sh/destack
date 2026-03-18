use std::sync::Arc;

use crate::host::core::registry::{HostRegistrationGuard, next_host_runtime_id};
use crate::host::core::{HostQueue, HostRuntimeRegistry};
use crate::host::ios::{
    IosHostBindings, IosHostCalendarCallbacks, destack_host_ios_calendar_event_create,
    destack_host_ios_calendar_event_delete, destack_host_ios_calendar_event_list,
    destack_host_ios_calendar_event_read, destack_host_ios_calendar_event_update,
    destack_host_ios_calendar_list, destack_host_ios_register_bindings, unregister_ios_bindings,
};
use crate::host::{HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK, Platform};
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
};
use crate::platform::os::{CalendarAccess, CalendarAvailability};
use crate::runtime::NativeSlice;

/// Register one temporary iOS host queue and keep registration state alive.
fn register_ios_runtime() -> (Arc<HostQueue>, HostRegistrationGuard, u64) {
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::downgrade(&queue),
        Some(unregister_ios_bindings),
    );
    let runtime_id = registration.host_runtime_id().0;

    (queue, registration, runtime_id)
}

/// Register one runtime-scoped iOS host bindings payload.
fn register_ios_bindings(runtime_id: u64, bindings: IosHostBindings) -> u32 {
    unsafe { destack_host_ios_register_bindings(runtime_id, bindings) }
}

unsafe extern "C" fn test_calendar_list_callback(
    _runtime_id: u64,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    write_json_output(output, output_written, &vec![test_calendar_descriptor()])
}

unsafe extern "C" fn test_calendar_event_list_callback(
    _runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let payload = unsafe { payload.as_slice() }.unwrap();
    let query = serde_json::from_slice::<CalendarEventQueryValue>(payload).unwrap();

    assert_eq!(query, test_calendar_query());

    write_json_output(output, output_written, &vec![test_calendar_event()])
}

unsafe extern "C" fn test_calendar_event_read_callback(
    _runtime_id: u64,
    id: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let id = unsafe { id.as_slice() }.unwrap();
    assert_eq!(std::str::from_utf8(id).unwrap(), "event-1");

    write_json_output(output, output_written, &test_calendar_event())
}

unsafe extern "C" fn test_calendar_event_create_callback(
    _runtime_id: u64,
    payload: NativeSlice<u8>,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let payload = unsafe { payload.as_slice() }.unwrap();
    let event = serde_json::from_slice::<CalendarEventDraftValue>(payload).unwrap();

    assert_eq!(event, test_calendar_event_draft());

    write_string_output(output_id, output_written, "event-1")
}

unsafe extern "C" fn test_calendar_event_update_callback(
    _runtime_id: u64,
    id: NativeSlice<u8>,
    payload: NativeSlice<u8>,
) -> u32 {
    let id = unsafe { id.as_slice() }.unwrap();
    let payload = unsafe { payload.as_slice() }.unwrap();
    let event = serde_json::from_slice::<CalendarEventDraftValue>(payload).unwrap();

    assert_eq!(std::str::from_utf8(id).unwrap(), "event-1");
    assert_eq!(event, test_calendar_event_draft());

    HOST_STATUS_OK
}

unsafe extern "C" fn test_calendar_event_delete_callback(
    _runtime_id: u64,
    id: NativeSlice<u8>,
) -> u32 {
    let id = unsafe { id.as_slice() }.unwrap();
    assert_eq!(std::str::from_utf8(id).unwrap(), "event-1");

    HOST_STATUS_OK
}

#[test]
fn test_calendar_callbacks_report_missing_runtime_registration() {
    let runtime_id = next_host_runtime_id().0;
    let status =
        unsafe { destack_host_ios_calendar_list(runtime_id, empty_output(), std::ptr::null_mut()) };

    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_calendar_callbacks_report_unsupported_without_registered_handler() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(runtime_id, IosHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let status =
        unsafe { destack_host_ios_calendar_list(runtime_id, empty_output(), std::ptr::null_mut()) };
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

    let mut list_output = vec![0_u8; 512];
    let mut list_written = 0_u32;
    let list_status = unsafe {
        destack_host_ios_calendar_list(
            runtime_id,
            NativeSlice {
                data: list_output.as_mut_ptr(),
                len: list_output.len() as u32,
            },
            &mut list_written,
        )
    };
    assert_eq!(list_status, HOST_STATUS_OK);
    let calendars = serde_json::from_slice::<Vec<CalendarDescriptorValue>>(
        &list_output[..list_written as usize],
    )
    .unwrap();
    assert_eq!(calendars, vec![test_calendar_descriptor()]);

    let query = serde_json::to_vec(&test_calendar_query()).unwrap();
    let mut event_list_output = vec![0_u8; 1024];
    let mut event_list_written = 0_u32;
    let event_list_status = unsafe {
        destack_host_ios_calendar_event_list(
            runtime_id,
            NativeSlice {
                data: query.as_ptr() as *mut u8,
                len: query.len() as u32,
            },
            NativeSlice {
                data: event_list_output.as_mut_ptr(),
                len: event_list_output.len() as u32,
            },
            &mut event_list_written,
        )
    };
    assert_eq!(event_list_status, HOST_STATUS_OK);
    let events = serde_json::from_slice::<Vec<CalendarEventValue>>(
        &event_list_output[..event_list_written as usize],
    )
    .unwrap();
    assert_eq!(events, vec![test_calendar_event()]);

    let event_id = b"event-1";
    let mut event_read_output = vec![0_u8; 1024];
    let mut event_read_written = 0_u32;
    let event_read_status = unsafe {
        destack_host_ios_calendar_event_read(
            runtime_id,
            NativeSlice {
                data: event_id.as_ptr() as *mut u8,
                len: event_id.len() as u32,
            },
            NativeSlice {
                data: event_read_output.as_mut_ptr(),
                len: event_read_output.len() as u32,
            },
            &mut event_read_written,
        )
    };
    assert_eq!(event_read_status, HOST_STATUS_OK);
    let event = serde_json::from_slice::<CalendarEventValue>(
        &event_read_output[..event_read_written as usize],
    )
    .unwrap();
    assert_eq!(event, test_calendar_event());

    let draft = serde_json::to_vec(&test_calendar_event_draft()).unwrap();
    let mut create_output = vec![0_u8; 128];
    let mut create_written = 0_u32;
    let create_status = unsafe {
        destack_host_ios_calendar_event_create(
            runtime_id,
            NativeSlice {
                data: draft.as_ptr() as *mut u8,
                len: draft.len() as u32,
            },
            NativeSlice {
                data: create_output.as_mut_ptr(),
                len: create_output.len() as u32,
            },
            &mut create_written,
        )
    };
    assert_eq!(create_status, HOST_STATUS_OK);
    assert_eq!(
        std::str::from_utf8(&create_output[..create_written as usize]).unwrap(),
        "event-1"
    );

    let update_status = unsafe {
        destack_host_ios_calendar_event_update(
            runtime_id,
            NativeSlice {
                data: event_id.as_ptr() as *mut u8,
                len: event_id.len() as u32,
            },
            NativeSlice {
                data: draft.as_ptr() as *mut u8,
                len: draft.len() as u32,
            },
        )
    };
    assert_eq!(update_status, HOST_STATUS_OK);

    let delete_status = unsafe {
        destack_host_ios_calendar_event_delete(
            runtime_id,
            NativeSlice {
                data: event_id.as_ptr() as *mut u8,
                len: event_id.len() as u32,
            },
        )
    };
    assert_eq!(delete_status, HOST_STATUS_OK);
}

fn empty_output() -> NativeSlice<u8> {
    NativeSlice {
        data: std::ptr::null_mut(),
        len: 0,
    }
}

fn write_json_output<T: serde::Serialize>(
    output: NativeSlice<u8>,
    output_written: *mut u32,
    value: &T,
) -> u32 {
    let bytes = serde_json::to_vec(value).unwrap();
    write_bytes_output(output, output_written, &bytes)
}

fn write_string_output(output: NativeSlice<u8>, output_written: *mut u32, value: &str) -> u32 {
    write_bytes_output(output, output_written, value.as_bytes())
}

fn write_bytes_output(output: NativeSlice<u8>, output_written: *mut u32, bytes: &[u8]) -> u32 {
    let output_written = unsafe { &mut *output_written };

    if output.len < bytes.len() as u32 {
        *output_written = bytes.len() as u32;
        return crate::host::HOST_STATUS_BUFFER_TOO_SMALL;
    }

    let output = unsafe { output.as_mut_slice() }.unwrap();
    output[..bytes.len()].copy_from_slice(bytes);
    *output_written = bytes.len() as u32;

    HOST_STATUS_OK
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
