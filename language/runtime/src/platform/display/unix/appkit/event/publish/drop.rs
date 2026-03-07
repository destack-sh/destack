use std::sync::Arc;

use crate::platform::display::WindowPosition;
use crate::platform::display::unix::appkit::core::AppKitRuntimeState;
use crate::platform::resource;

use crate::platform::display::unix::appkit::event::queue::publish_window_event;
use crate::platform::display::unix::appkit::event::{WindowEventRecordKind, window_event_record};

/// Publish one drop-started window event.
#[allow(dead_code)]
pub(crate) fn publish_window_drop_started(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropStarted { window }),
    );
}

/// Publish one file-hovered window event.
#[allow(dead_code)]
pub(crate) fn publish_window_file_hovered(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    path: Option<String>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileHovered {
            window,
            path,
            position,
        }),
    );
}

/// Publish one drop-cancelled window event.
#[allow(dead_code)]
pub(crate) fn publish_window_drop_cancelled(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropCancelled { window }),
    );
}

/// Publish one drop-completed window event.
#[allow(dead_code)]
pub(crate) fn publish_window_drop_completed(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DropCompleted { window }),
    );
}

/// Publish one file-hover-left window event.
#[allow(dead_code)]
pub(crate) fn publish_window_file_hover_left(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    previous_path: Option<String>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileHoverLeft {
            window,
            previous_path,
            position,
        }),
    );
}

/// Publish one file-dropped window event.
#[allow(dead_code)]
pub(crate) fn publish_window_file_dropped(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    path: Option<String>,
    position: Option<WindowPosition>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FileDropped {
            window,
            path,
            position,
        }),
    );
}

/// Publish one text-dropped window event.
#[allow(dead_code)]
pub(crate) fn publish_window_text_dropped(
    runtime_state: &Arc<AppKitRuntimeState>,
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
