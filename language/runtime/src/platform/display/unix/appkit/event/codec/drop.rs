use crate::platform::display::{
    WindowDragEnteredEvent, WindowDragExitedEvent, WindowDragUpdatedEvent, WindowDroppedEvent,
    WindowEvent, core as display_core,
};
use crate::runtime::BindingCallContext;

use super::core::window_event_metadata;
use crate::platform::display::unix::appkit::event::{WindowEventRecord, WindowEventRecordKind};

/// Convert one drag-lane window-event record into one ABI event payload.
pub(crate) fn window_drag_event_from_record(
    context: &BindingCallContext,
    value: WindowEventRecord,
) -> WindowEvent {
    match value.kind {
        WindowEventRecordKind::DragEntered { window, session } => {
            WindowEvent::WindowDragEnteredEvent(WindowDragEnteredEvent {
                kind: context.store_string("dragEntered"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: display_core::drag_transfer(context, session).unwrap_or_else(|error| {
                    panic!("failed to resolve drag session for dragEntered: {error}")
                }),
            })
        }
        WindowEventRecordKind::DragUpdated { window, session } => {
            WindowEvent::WindowDragUpdatedEvent(WindowDragUpdatedEvent {
                kind: context.store_string("dragUpdated"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: display_core::drag_transfer(context, session).unwrap_or_else(|error| {
                    panic!("failed to resolve drag session for dragUpdated: {error}")
                }),
            })
        }
        WindowEventRecordKind::DragExited { window, session } => {
            WindowEvent::WindowDragExitedEvent(WindowDragExitedEvent {
                kind: context.store_string("dragExited"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: display_core::drag_transfer(context, session).unwrap_or_else(|error| {
                    panic!("failed to resolve drag session for dragExited: {error}")
                }),
            })
        }
        WindowEventRecordKind::Dropped { window, session } => {
            WindowEvent::WindowDroppedEvent(WindowDroppedEvent {
                kind: context.store_string("dropped"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: display_core::drag_transfer(context, session).unwrap_or_else(|error| {
                    panic!("failed to resolve drag session for dropped: {error}")
                }),
            })
        }

        // non-drag events are routed through the state codec
        WindowEventRecordKind::Created { .. }
        | WindowEventRecordKind::CloseRequested { .. }
        | WindowEventRecordKind::Destroyed { .. }
        | WindowEventRecordKind::RefreshRequested { .. }
        | WindowEventRecordKind::VisibilityChanged { .. }
        | WindowEventRecordKind::OcclusionChanged { .. }
        | WindowEventRecordKind::PositionChanged { .. }
        | WindowEventRecordKind::SizeChanged { .. }
        | WindowEventRecordKind::FocusChanged { .. }
        | WindowEventRecordKind::ModeChanged { .. }
        | WindowEventRecordKind::ScaleFactorChanged { .. }
        | WindowEventRecordKind::DisplayChanged { .. }
        | WindowEventRecordKind::ThemeChanged { .. }
        | WindowEventRecordKind::ChromeChanged { .. }
        | WindowEventRecordKind::TaskbarVisibilityChanged { .. }
        | WindowEventRecordKind::SafeAreaChanged { .. }
        | WindowEventRecordKind::OpacityChanged { .. }
        | WindowEventRecordKind::ParentChanged { .. }
        | WindowEventRecordKind::TransientChanged { .. }
        | WindowEventRecordKind::ModalChanged { .. }
        | WindowEventRecordKind::MousePassthroughChanged { .. }
        | WindowEventRecordKind::AspectRatioChanged { .. } => unreachable!(),
    }
}
