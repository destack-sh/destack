use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::{
    DisplayEventRecordKind, MonitorEventFilterState, MonitorEventState, MonitorEventStream,
    WindowEventFilterState, WindowEventRecordKind, WindowEventState, WindowEventStream,
    display_event_record,
};
use crate::platform::display::{
    DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED, DisplayBackend, DisplayEventOverflowPolicy,
    DisplayMode, DisplayOrientation, DisplaySupportStatus, WINDOW_EVENT_KIND_DROP_STARTED,
    WINDOW_EVENT_KIND_TEXT_DROPPED, WindowPosition,
};
use crate::platform::resource::{ResourceId, WindowHandle};

use crate::platform::display::windows::win32::DISPLAY_CHANGED_MASK_BOUNDS;
use crate::platform::display::windows::win32::core::Win32RuntimeState;
use crate::platform::display::windows::win32::event::codec::monitor_topology_records;
use crate::platform::display::windows::win32::event::publish::{
    publish_window_drop_cancelled_event, publish_window_drop_completed_event,
    publish_window_drop_started_event, publish_window_file_dropped_event,
    publish_window_text_dropped_event,
};
use crate::platform::display::windows::win32::event::queue::{
    pop_live_monitor_record, pop_live_window_record, publish_monitor_event,
};
use crate::platform::display::windows::win32::model::{DisplayDescriptorSnapshot, MonitorSnapshot};

/// Build one test descriptor payload with explicit geometry fields.
fn descriptor(id: &str, primary: bool, width_px: u32, height_px: u32) -> DisplayDescriptorSnapshot {
    DisplayDescriptorSnapshot {
        backend: DisplayBackend::Win32,
        id: id.to_string(),
        name: id.to_string(),
        primary,
        x: 0,
        y: 0,
        width_px,
        height_px,
        work_area_x: 0,
        work_area_y: 0,
        work_area_width_px: width_px,
        work_area_height_px: height_px,
        width_mm: 500,
        height_mm: 300,
        scale_factor_milli: 1000,
        orientation: DisplayOrientation::Landscape,
        builtin_panel: DisplaySupportStatus::Unsupported,
        variable_refresh_support: DisplaySupportStatus::Unknown,
        hdr_support: DisplaySupportStatus::Unsupported,
    }
}

/// Build one test mode payload.
fn mode(width: u32, height: u32, refresh_milli_hz: u32) -> DisplayMode {
    DisplayMode {
        width,
        height,
        refresh_milli_hz,
        format: 0,
        bit_depth: 8,
    }
}

/// Build one monitor snapshot payload for tests.
fn snapshot(descriptor: DisplayDescriptorSnapshot, mode: DisplayMode) -> MonitorSnapshot {
    MonitorSnapshot {
        descriptor,
        current_mode: mode,
        desktop_mode: mode,
        modes: vec![mode],
    }
}

/// Monitor topology records should include removed, added, mode, and primary events.
#[test]
fn test_monitor_topology_records_emit_removed_added_and_primary_changed() {
    let previous = vec![snapshot(
        descriptor(r"\\.\DISPLAY1", true, 1920, 1080),
        mode(1920, 1080, 60_000),
    )];
    let next = vec![snapshot(
        descriptor(r"\\.\DISPLAY2", true, 2560, 1440),
        mode(2560, 1440, 144_000),
    )];

    let records = monitor_topology_records(&previous, &next);

    assert!(records.iter().any(|record| matches!(
        record.kind,
        DisplayEventRecordKind::Removed { ref id, .. } if id == r"\\.\DISPLAY1"
    )));
    assert!(records.iter().any(|record| matches!(
        record.kind,
        DisplayEventRecordKind::Added { ref descriptor } if descriptor.id == r"\\.\DISPLAY2"
    )));
    assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::ModeChanged { ref id, current: mode, .. } if id == r"\\.\DISPLAY2" && mode.refresh_milli_hz == 144_000
        )));
    assert!(records.iter().any(|record| matches!(
            record.kind,
            DisplayEventRecordKind::PrimaryChanged { current_id: Some(ref id), .. } if id == r"\\.\DISPLAY2"
        )));
}

/// Monitor topology records should include descriptor and mode deltas for one existing display.
#[test]
fn test_monitor_topology_records_emit_descriptor_and_mode_changes() {
    let previous = vec![snapshot(
        descriptor(r"\\.\DISPLAY1", true, 1920, 1080),
        mode(1920, 1080, 60_000),
    )];
    let next = vec![snapshot(
        descriptor(r"\\.\DISPLAY1", true, 2560, 1440),
        mode(2560, 1440, 120_000),
    )];

    let records = monitor_topology_records(&previous, &next);

    assert!(records.iter().any(|record| matches!(
        record.kind,
        DisplayEventRecordKind::DescriptorChanged { ref current, changed_mask, .. }
            if current.id == r"\\.\DISPLAY1" && (changed_mask & DISPLAY_CHANGED_MASK_BOUNDS) != 0
    )));
    assert!(records.iter().any(|record| matches!(
        record.kind,
        DisplayEventRecordKind::ModeChanged { ref id, current: mode, .. }
            if id == r"\\.\DISPLAY1" && mode.refresh_milli_hz == 120_000
    )));
}

/// Build one runtime state and one subscribed window-event stream for publication tests.
fn subscribed_window_stream_with_filter(
    filter: WindowEventFilterState,
) -> (Arc<Win32RuntimeState>, Arc<WindowEventStream>, WindowHandle) {
    let runtime_state = Arc::new(Win32RuntimeState::default());
    let next_live_sequence = {
        let window_events = runtime_state
            .window_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        window_events.next_sequence
    };
    let window_event_stream = Arc::new(WindowEventStream {
        stream_id: runtime_state.next_window_stream_id(),
        state: Mutex::new(WindowEventState {
            queue_capacity: 32,
            overflow_policy: DisplayEventOverflowPolicy::DropOldest,
            overflow_error_pending: false,
            next_output_sequence: 1,
            dropped_count: 0,
            next_live_sequence,
            unread_live_count: 0,
            seeded: VecDeque::new(),
        }),
        filter,
    });
    runtime_state.register_window_stream(Arc::clone(&window_event_stream));
    let window = WindowHandle(ResourceId::local(123));

    (runtime_state, window_event_stream, window)
}

/// Build one runtime state and one subscribed window-event stream for publication tests.
fn subscribed_window_stream() -> (Arc<Win32RuntimeState>, Arc<WindowEventStream>, WindowHandle) {
    subscribed_window_stream_with_filter(WindowEventFilterState::default())
}

/// Build one runtime state and one subscribed monitor-event stream for publication tests.
fn subscribed_monitor_stream_with_filter(
    filter: MonitorEventFilterState,
) -> (Arc<Win32RuntimeState>, Arc<MonitorEventStream>) {
    let runtime_state = Arc::new(Win32RuntimeState::default());
    let next_live_sequence = {
        let monitor_events = runtime_state
            .monitor_events
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        monitor_events.next_sequence
    };
    let monitor_event_stream = Arc::new(MonitorEventStream {
        stream_id: runtime_state.next_monitor_stream_id(),
        state: Mutex::new(MonitorEventState {
            queue_capacity: 32,
            overflow_policy: DisplayEventOverflowPolicy::DropOldest,
            overflow_error_pending: false,
            next_output_sequence: 1,
            dropped_count: 0,
            next_live_sequence,
            unread_live_count: 0,
            seeded: VecDeque::new(),
        }),
        filter,
    });
    runtime_state.register_monitor_stream(Arc::clone(&monitor_event_stream));

    (runtime_state, monitor_event_stream)
}

/// Drop lifecycle publishers should enqueue one started then completed event sequence.
#[test]
fn test_drop_lifecycle_publishers_enqueue_expected_record_kinds() {
    let (runtime_state, window_event_stream, window) = subscribed_window_stream();

    publish_window_drop_started_event(&runtime_state, window);
    publish_window_drop_completed_event(&runtime_state, window);
    publish_window_drop_cancelled_event(&runtime_state, window);

    let first = pop_live_window_record(&runtime_state, &window_event_stream)
        .expect("missing dropStarted record");
    let second = pop_live_window_record(&runtime_state, &window_event_stream)
        .expect("missing dropCompleted record");
    let third = pop_live_window_record(&runtime_state, &window_event_stream)
        .expect("missing dropCancelled record");

    assert!(matches!(
        first.kind,
        WindowEventRecordKind::DropStarted { window: value } if value == window
    ));
    assert!(matches!(
        second.kind,
        WindowEventRecordKind::DropCompleted { window: value } if value == window
    ));
    assert!(matches!(
        third.kind,
        WindowEventRecordKind::DropCancelled { window: value } if value == window
    ));
}

/// File-drop publisher should preserve UTF-16 path payload and position metadata.
#[test]
fn test_file_drop_publisher_preserves_path_and_position_payloads() {
    let (runtime_state, window_event_stream, window) = subscribed_window_stream();
    let path_utf16 = "C:\\drop\\asset.txt".encode_utf16().collect::<Vec<_>>();
    let position = Some(WindowPosition { x: 480, y: 320 });

    publish_window_file_dropped_event(&runtime_state, window, Some(path_utf16.clone()), position);

    let record = pop_live_window_record(&runtime_state, &window_event_stream)
        .expect("missing fileDropped record");
    assert!(matches!(
        record.kind,
        WindowEventRecordKind::FileDropped {
            window: value,
            path_utf16: Some(path),
            position: payload_position,
        } if value == window && path == path_utf16 && payload_position == position
    ));
}

/// Text-drop publisher should preserve text payload and position metadata.
#[test]
fn test_text_drop_publisher_preserves_text_and_position_payloads() {
    let (runtime_state, window_event_stream, window) = subscribed_window_stream();
    let text_payload = String::from("dropped-text");
    let position = Some(WindowPosition { x: 640, y: 480 });

    publish_window_text_dropped_event(&runtime_state, window, text_payload.clone(), position);

    let record = pop_live_window_record(&runtime_state, &window_event_stream)
        .expect("missing textDropped record");
    assert!(matches!(
        record.kind,
        WindowEventRecordKind::TextDropped {
            window: value,
            text,
            position: payload_position,
        } if value == window && text == text_payload && payload_position == position
    ));
}

/// Window-event filters should restrict delivery to one target window.
#[test]
fn test_window_event_filter_restricts_window_handle() {
    let target_window = WindowHandle(ResourceId::local(200));
    let other_window = WindowHandle(ResourceId::local(201));
    let (runtime_state, stream, _) = subscribed_window_stream_with_filter(
        WindowEventFilterState::new(Some(target_window), None),
    );

    publish_window_drop_started_event(&runtime_state, other_window);
    publish_window_drop_started_event(&runtime_state, target_window);

    let state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert_eq!(state.unread_live_count, 1);
    drop(state);
    assert!(matches!(
        pop_live_window_record(&runtime_state, &stream).map(|value| value.kind),
        Some(WindowEventRecordKind::DropStarted { window }) if window == target_window
    ));
}

/// Window-event filters should restrict delivery by event-kind mask.
#[test]
fn test_window_event_filter_restricts_kind_mask() {
    let (runtime_state, stream, window) = subscribed_window_stream_with_filter(
        WindowEventFilterState::new(None, Some(WINDOW_EVENT_KIND_DROP_STARTED.0)),
    );

    publish_window_drop_started_event(&runtime_state, window);
    publish_window_drop_completed_event(&runtime_state, window);

    let state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert_eq!(state.unread_live_count, 1);
    drop(state);
    assert!(matches!(
        pop_live_window_record(&runtime_state, &stream).map(|value| value.kind),
        Some(WindowEventRecordKind::DropStarted { window: value }) if value == window
    ));
}

/// Window-event kind filters should route only text-dropped records.
#[test]
fn test_window_event_filter_accepts_text_dropped_kind() {
    let (runtime_state, stream, window) = subscribed_window_stream_with_filter(
        WindowEventFilterState::new(None, Some(WINDOW_EVENT_KIND_TEXT_DROPPED.0)),
    );

    publish_window_drop_started_event(&runtime_state, window);
    publish_window_text_dropped_event(
        &runtime_state,
        window,
        String::from("accepted-text"),
        Some(WindowPosition { x: 11, y: 22 }),
    );

    let state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert_eq!(state.unread_live_count, 1);
    drop(state);
    assert!(matches!(
        pop_live_window_record(&runtime_state, &stream).map(|value| value.kind),
        Some(WindowEventRecordKind::TextDropped {
            window: value,
            text,
            position: Some(WindowPosition { x: 11, y: 22 })
        }) if value == window && text == "accepted-text"
    ));
}

/// Monitor-event filters should restrict delivery by event-kind mask.
#[test]
fn test_monitor_event_filter_restricts_kind_mask() {
    let (runtime_state, stream) = subscribed_monitor_stream_with_filter(
        MonitorEventFilterState::new(None, Some(DISPLAY_MONITOR_EVENT_KIND_MODE_CHANGED.0)),
    );

    publish_monitor_event(
        &runtime_state,
        display_event_record(DisplayEventRecordKind::PrimaryChanged {
            previous_id: Some(String::from(r"\\.\DISPLAY1")),
            current_id: Some(String::from(r"\\.\DISPLAY2")),
        }),
    );
    publish_monitor_event(
        &runtime_state,
        display_event_record(DisplayEventRecordKind::ModeChanged {
            id: String::from(r"\\.\DISPLAY2"),
            previous: None,
            current: mode(2560, 1440, 144_000),
        }),
    );

    let state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert_eq!(state.unread_live_count, 1);
    drop(state);
    assert!(matches!(
        pop_live_monitor_record(&runtime_state, &stream).map(|value| value.kind),
        Some(DisplayEventRecordKind::ModeChanged { ref id, .. }) if id == r"\\.\DISPLAY2"
    ));
}

/// Monitor-event filters should restrict delivery by display identifier.
#[test]
fn test_monitor_event_filter_restricts_display_identifier() {
    let (runtime_state, stream) = subscribed_monitor_stream_with_filter(
        MonitorEventFilterState::new(Some(String::from(r"\\.\DISPLAY2")), None),
    );

    publish_monitor_event(
        &runtime_state,
        display_event_record(DisplayEventRecordKind::Added {
            descriptor: descriptor(r"\\.\DISPLAY1", false, 1920, 1080),
        }),
    );
    publish_monitor_event(
        &runtime_state,
        display_event_record(DisplayEventRecordKind::Added {
            descriptor: descriptor(r"\\.\DISPLAY2", true, 2560, 1440),
        }),
    );

    let state = stream
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert_eq!(state.unread_live_count, 1);
    drop(state);
    assert!(matches!(
        pop_live_monitor_record(&runtime_state, &stream).map(|value| value.kind),
        Some(DisplayEventRecordKind::Added { descriptor }) if descriptor.id == r"\\.\DISPLAY2"
    ));
}
