use std::sync::Arc;

use crate::platform::display::unix::appkit::core::AppKitRuntimeState;
use crate::platform::resource;

use crate::platform::display::unix::appkit::event::queue::publish_window_event;
use crate::platform::display::unix::appkit::event::{WindowEventRecordKind, window_event_record};

/// Publish one drag-entered window event.
#[allow(dead_code)]
pub(crate) fn publish_window_drag_entered(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    session: resource::DisplayDragSessionHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DragEntered { window, session }),
    );
}

/// Publish one drag-updated window event.
#[allow(dead_code)]
pub(crate) fn publish_window_drag_updated(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    session: resource::DisplayDragSessionHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DragUpdated { window, session }),
    );
}

/// Publish one drag-exited window event.
#[allow(dead_code)]
pub(crate) fn publish_window_drag_exited(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    session: resource::DisplayDragSessionHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DragExited { window, session }),
    );
}

/// Publish one dropped window event.
#[allow(dead_code)]
pub(crate) fn publish_window_dropped(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    session: resource::DisplayDragSessionHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Dropped { window, session }),
    );
}
