use std::sync::Arc;

use crate::platform::display::WindowPosition;
use crate::platform::display::windows::win32::core::Win32RuntimeState;
use crate::platform::display::windows::win32::event::queue::publish_window_event;
use crate::platform::display::windows::win32::event::{WindowEventRecordKind, window_event_record};
use crate::platform::resource;

/// Publish one drop-started window event.
pub(crate) fn publish_window_drop_started(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropStarted { window }),
    );
}

/// Publish one file-hovered window event.
pub(crate) fn publish_window_file_hovered(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    path_utf16: Option<Vec<u16>>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileHovered {
            window,
            path_utf16,
            position,
        }),
    );
}

/// Publish one drop-cancelled window event.
pub(crate) fn publish_window_drop_cancelled(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropCancelled { window }),
    );
}

/// Publish one drop-completed window event.
pub(crate) fn publish_window_drop_completed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropCompleted { window }),
    );
}

/// Publish one file-hover-left window event.
pub(crate) fn publish_window_file_hover_left(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_path_utf16: Option<Vec<u16>>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileHoverLeft {
            window,
            previous_path_utf16,
            position,
        }),
    );
}

/// Publish one file-dropped window event.
pub(crate) fn publish_window_file_dropped(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    path_utf16: Option<Vec<u16>>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileDropped {
            window,
            path_utf16,
            position,
        }),
    );
}

/// Publish one text-dropped window event.
pub(crate) fn publish_window_text_dropped(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    text: String,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::TextDropped {
            window,
            text,
            position,
        }),
    );
}
