use std::sync::Arc;

use crate::platform::display::windows::win32::core::Win32RuntimeState;
use crate::platform::display::windows::win32::event::queue::publish_window_event;
use crate::platform::display::windows::win32::event::{WindowEventRecordKind, window_event_record};
use crate::platform::display::{
    WindowLogicalSize, WindowModeOptions, WindowPhysicalSize, WindowPosition, WindowVisibility,
};
use crate::platform::resource;

/// Publish one created window event.
pub(crate) fn publish_window_created(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Created { window }),
    );
}

/// Publish one destroyed window event.
pub(crate) fn publish_window_destroyed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::Destroyed { window }),
    );
}

/// Publish one close-requested window event.
pub(crate) fn publish_window_close_requested(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::CloseRequested { window }),
    );
}

/// Publish one refresh-requested window event.
pub(crate) fn publish_window_refresh_requested(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::RefreshRequested { window }),
    );
}

/// Publish one visibility-changed window event.
pub(crate) fn publish_window_visibility_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_visibility: WindowVisibility,
    current_visibility: WindowVisibility,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::VisibilityChanged {
            window,
            previous_visibility,
            current_visibility,
        }),
    );
}

/// Publish one position-changed window event.
pub(crate) fn publish_window_position_changed(
    runtime_state: &Arc<Win32RuntimeState>,
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
    runtime_state: &Arc<Win32RuntimeState>,
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

/// Publish one scale-factor changed window event.
pub(crate) fn publish_window_scale_factor_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_scale_factor_milli: u32,
    current_scale_factor_milli: u32,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::ScaleFactorChanged {
            window,
            previous_scale_factor_milli,
            current_scale_factor_milli,
        }),
    );
}

/// Publish one mode-changed window event.
pub(crate) fn publish_window_mode_changed(
    runtime_state: &Arc<Win32RuntimeState>,
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
