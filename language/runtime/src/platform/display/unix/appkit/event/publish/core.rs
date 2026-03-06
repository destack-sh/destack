use std::sync::Arc;

use crate::platform::display::host::unix::appkit::core::AppKitRuntimeState;
use crate::platform::display::{
    WindowLogicalSize, WindowModeOptions, WindowOcclusionState, WindowPhysicalSize, WindowPosition,
    WindowVisibility,
};
use crate::platform::resource;

use super::super::queue::publish_window_event;
use super::super::{WindowEventRecordKind, window_event_record};

/// Resolve one occlusion state from one visibility value.
pub(crate) fn occlusion_from_visibility(visibility: WindowVisibility) -> WindowOcclusionState {
    // hidden and minimized windows are treated as occluded
    if visibility == WindowVisibility::Hidden || visibility == WindowVisibility::Minimized {
        return WindowOcclusionState::Occluded;
    }

    WindowOcclusionState::Unknown
}

/// Publish one created window event.
pub(crate) fn publish_window_created(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Created { window }),
    );
}

/// Publish one close-requested window event.
pub(crate) fn publish_window_close_requested(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::CloseRequested { window }),
    );
}

/// Publish one destroyed window event.
pub(crate) fn publish_window_destroyed(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Destroyed { window }),
    );
}

/// Publish one refresh-requested window event.
pub(crate) fn publish_window_refresh_requested(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::RefreshRequested { window }),
    );
}

/// Publish one visibility-changed window event.
pub(crate) fn publish_window_visibility_changed(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    previous_visibility: WindowVisibility,
    current_visibility: WindowVisibility,
) {
    let previous_occlusion = occlusion_from_visibility(previous_visibility);
    let current_occlusion = occlusion_from_visibility(current_visibility);

    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::VisibilityChanged {
            window,
            previous_visibility,
            current_visibility,
        }),
    );

    // publish one occlusion transition when visibility implies one state change
    if previous_occlusion != current_occlusion {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::OcclusionChanged {
                window,
                previous_occlusion,
                current_occlusion,
            }),
        );
    }
}

/// Publish one position-changed window event.
pub(crate) fn publish_window_position_changed(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    previous_position: WindowPosition,
    current_position: WindowPosition,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::PositionChanged {
            window,
            previous_position,
            current_position,
        }),
    );
}

/// Publish one size-changed window event.
pub(crate) fn publish_window_size_changed(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    previous_size_logical: WindowLogicalSize,
    previous_size_physical: WindowPhysicalSize,
    current_size_logical: WindowLogicalSize,
    current_size_physical: WindowPhysicalSize,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::SizeChanged {
            window,
            previous_size_logical,
            previous_size_physical,
            current_size_logical,
            current_size_physical,
        }),
    );
}

/// Publish one focus-changed window event.
pub(crate) fn publish_window_focus_changed(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    previous_focused: bool,
    current_focused: bool,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::FocusChanged {
            window,
            previous_focused,
            current_focused,
        }),
    );
}

/// Publish one mode-changed window event.
pub(crate) fn publish_window_mode_changed(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    previous_mode: WindowModeOptions,
    current_mode: WindowModeOptions,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::ModeChanged {
            window,
            previous_mode,
            current_mode,
        }),
    );
}
