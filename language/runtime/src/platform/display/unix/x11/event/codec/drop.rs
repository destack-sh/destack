use crate::platform::display::unix::x11::event::{WindowEventRecord, WindowEventRecordKind};
use crate::platform::display::{
    WindowDropCancelledEvent, WindowDropCompletedEvent, WindowDropFilePayload,
    WindowDropHoverLeavePayload, WindowDropHoverPayload, WindowDropStartedEvent,
    WindowDropTextPayload, WindowEvent, WindowFileDroppedEvent, WindowFileHoverLeftEvent,
    WindowFileHoveredEvent, WindowTextDroppedEvent,
};
use crate::runtime::BindingCallContext;

use super::core::{os_path_from_utf8, window_event_metadata};

/// Convert one drop-lane window-event record into one ABI event payload.
pub(crate) fn window_drop_event_from_record(
    context: &BindingCallContext,
    value: WindowEventRecord,
) -> WindowEvent {
    // decode this drop-event variant
    match value.kind {
        WindowEventRecordKind::DropStarted { window } => {
            WindowEvent::WindowDropStartedEvent(WindowDropStartedEvent {
                kind: context.store_string("dropStarted"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::FileHovered {
            window,
            path,
            position,
        } => WindowEvent::WindowFileHoveredEvent(WindowFileHoveredEvent {
            kind: context.store_string("fileHovered"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropHoverPayload {
                path: path.as_ref().map(|value| os_path_from_utf8(context, value)),
                position,
            },
        }),
        WindowEventRecordKind::DropCancelled { window } => {
            WindowEvent::WindowDropCancelledEvent(WindowDropCancelledEvent {
                kind: context.store_string("dropCancelled"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::DropCompleted { window } => {
            WindowEvent::WindowDropCompletedEvent(WindowDropCompletedEvent {
                kind: context.store_string("dropCompleted"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::FileHoverLeft {
            window,
            previous_path,
            position,
        } => WindowEvent::WindowFileHoverLeftEvent(WindowFileHoverLeftEvent {
            kind: context.store_string("fileHoverLeft"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropHoverLeavePayload {
                previous_path: previous_path
                    .as_ref()
                    .map(|value| os_path_from_utf8(context, value)),
                position,
            },
        }),
        WindowEventRecordKind::FileDropped {
            window,
            path,
            position,
        } => WindowEvent::WindowFileDroppedEvent(WindowFileDroppedEvent {
            kind: context.store_string("fileDropped"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropFilePayload {
                path: path.as_ref().map(|value| os_path_from_utf8(context, value)),
                position,
            },
        }),
        WindowEventRecordKind::TextDropped {
            window,
            text,
            position,
        } => WindowEvent::WindowTextDroppedEvent(WindowTextDroppedEvent {
            kind: context.store_string("textDropped"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropTextPayload {
                text: context.store_string(&text),
                position,
            },
        }),

        // non-drop events are routed through the state codec
        WindowEventRecordKind::Created { .. }
        | WindowEventRecordKind::CloseRequested { .. }
        | WindowEventRecordKind::Destroyed { .. }
        | WindowEventRecordKind::RefreshRequested { .. }
        | WindowEventRecordKind::VisibilityChanged { .. }
        | WindowEventRecordKind::OcclusionChanged { .. }
        | WindowEventRecordKind::PositionChanged { .. }
        | WindowEventRecordKind::SizeChanged { .. }
        | WindowEventRecordKind::ScaleFactorChanged { .. }
        | WindowEventRecordKind::FocusChanged { .. }
        | WindowEventRecordKind::ModeChanged { .. }
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
