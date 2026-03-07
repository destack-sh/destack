use crate::platform::display::windows::win32::event::{WindowEventRecord, WindowEventRecordKind};
use crate::platform::display::{
    WindowDropCancelledEvent, WindowDropCompletedEvent, WindowDropFilePayload,
    WindowDropHoverLeavePayload, WindowDropHoverPayload, WindowDropStartedEvent,
    WindowDropTextPayload, WindowEvent, WindowFileDroppedEvent, WindowFileHoverLeftEvent,
    WindowFileHoveredEvent, WindowTextDroppedEvent,
};
use crate::runtime::BindingCallContext;

use super::core::{os_path_from_utf16_units, window_event_metadata};

/// Convert one drop-lane window-event record into one ABI event payload.
pub(crate) fn window_drop_event_from_record(
    value: WindowEventRecord,
    binding: &BindingCallContext,
) -> WindowEvent {
    // decode this drop-event variant
    match value.kind {
        WindowEventRecordKind::DropStarted { window } => {
            WindowEvent::WindowDropStartedEvent(WindowDropStartedEvent {
                kind: binding.store_string("dropStarted"),
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
            path_utf16,
            position,
        } => WindowEvent::WindowFileHoveredEvent(WindowFileHoveredEvent {
            kind: binding.store_string("fileHovered"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropHoverPayload {
                path: path_utf16
                    .as_ref()
                    .map(|units| os_path_from_utf16_units(binding, units)),
                position,
            },
        }),
        WindowEventRecordKind::DropCancelled { window } => {
            WindowEvent::WindowDropCancelledEvent(WindowDropCancelledEvent {
                kind: binding.store_string("dropCancelled"),
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
                kind: binding.store_string("dropCompleted"),
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
            previous_path_utf16,
            position,
        } => WindowEvent::WindowFileHoverLeftEvent(WindowFileHoverLeftEvent {
            kind: binding.store_string("fileHoverLeft"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropHoverLeavePayload {
                previous_path: previous_path_utf16
                    .as_ref()
                    .map(|units| os_path_from_utf16_units(binding, units)),
                position,
            },
        }),
        WindowEventRecordKind::FileDropped {
            window,
            path_utf16,
            position,
        } => WindowEvent::WindowFileDroppedEvent(WindowFileDroppedEvent {
            kind: binding.store_string("fileDropped"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropFilePayload {
                path: path_utf16
                    .as_ref()
                    .map(|units| os_path_from_utf16_units(binding, units)),
                position,
            },
        }),
        WindowEventRecordKind::TextDropped {
            window,
            text,
            position,
        } => WindowEvent::WindowTextDroppedEvent(WindowTextDroppedEvent {
            kind: binding.store_string("textDropped"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDropTextPayload {
                text: binding.store_string(&text),
                position,
            },
        }),

        // non-drop events are routed through the state codec
        WindowEventRecordKind::Created { .. }
        | WindowEventRecordKind::CloseRequested { .. }
        | WindowEventRecordKind::Destroyed { .. }
        | WindowEventRecordKind::FocusChanged { .. }
        | WindowEventRecordKind::VisibilityChanged { .. }
        | WindowEventRecordKind::OcclusionChanged { .. }
        | WindowEventRecordKind::PositionChanged { .. }
        | WindowEventRecordKind::SizeChanged { .. }
        | WindowEventRecordKind::ScaleFactorChanged { .. }
        | WindowEventRecordKind::RefreshRequested { .. }
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
