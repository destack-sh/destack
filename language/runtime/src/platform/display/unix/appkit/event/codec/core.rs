use crate::platform::display::unix::appkit::core as appkit_core;
use crate::platform::display::{WindowEvent, WindowEventMetadata};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::display::unix::appkit::event::{WindowEventRecord, WindowEventRecordKind};

/// Build one window-event metadata payload.
pub(crate) fn window_event_metadata(
    window: resource::WindowHandle,
    timestamp_ns: u64,
    sequence: u64,
    dropped_count: u64,
) -> WindowEventMetadata {
    WindowEventMetadata {
        backend: appkit_core::selected_backend(),
        window,
        timestamp_ns,
        sequence,
        dropped_count,
    }
}

/// Convert one stored window-event record into one ABI event payload.
pub(crate) fn window_event_from_record(
    context: &BindingCallContext,
    value: WindowEventRecord,
) -> WindowEvent {
    // route drag records through the drag codec
    if matches!(
        value.kind,
        WindowEventRecordKind::DragEntered { .. }
            | WindowEventRecordKind::DragUpdated { .. }
            | WindowEventRecordKind::DragExited { .. }
            | WindowEventRecordKind::Dropped { .. }
    ) {
        return super::drop::window_drag_event_from_record(context, value);
    }

    super::state::window_state_event_from_record(context, value)
}
