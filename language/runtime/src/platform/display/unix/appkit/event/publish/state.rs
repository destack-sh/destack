use std::sync::Arc;

use crate::platform::display::unix::appkit::core::AppKitRuntimeState;
use crate::platform::display::unix::appkit::model::AppKitWindowHostState;
use crate::platform::display::unix::appkit::window;
use crate::platform::resource;

use super::core::{
    occlusion_from_visibility, publish_window_focus_changed, publish_window_mode_changed,
    publish_window_position_changed, publish_window_size_changed,
    publish_window_visibility_changed,
};
use crate::platform::display::unix::appkit::event::queue::publish_window_event;
use crate::platform::display::unix::appkit::event::{WindowEventRecordKind, window_event_record};

/// Publish all supported state-delta events between two AppKit window snapshots.
pub(crate) fn publish_state_deltas(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    previous: &AppKitWindowHostState,
    next: &AppKitWindowHostState,
) {
    // publish visibility transitions
    if previous.visibility != next.visibility {
        publish_window_visibility_changed(
            runtime_state,
            window,
            previous.visibility,
            next.visibility,
        );
    }

    let previous_occlusion = occlusion_from_visibility(previous.visibility);
    let current_occlusion = occlusion_from_visibility(next.visibility);

    // publish explicit occlusion changes that are not already implied by visibility
    if previous.occlusion != next.occlusion && previous_occlusion == current_occlusion {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::OcclusionChanged {
                window,
                previous_occlusion: previous.occlusion,
                current_occlusion: next.occlusion,
            }),
        );
    }

    // publish position transitions
    if previous.position != next.position {
        publish_window_position_changed(runtime_state, window, previous.position, next.position);
    }

    // publish logical or physical size transitions
    if previous.size_logical != next.size_logical || previous.size_physical != next.size_physical {
        publish_window_size_changed(
            runtime_state,
            window,
            previous.size_logical,
            previous.size_physical,
            next.size_logical,
            next.size_physical,
        );
    }

    // publish scale-factor transitions
    if previous.scale_factor_milli != next.scale_factor_milli {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::ScaleFactorChanged {
                window,
                previous_scale_factor_milli: previous.scale_factor_milli,
                current_scale_factor_milli: next.scale_factor_milli,
            }),
        );
    }

    // publish focus transitions
    if previous.focused != next.focused {
        publish_window_focus_changed(runtime_state, window, previous.focused, next.focused);
    }

    // publish mode transitions
    if !window::same_window_mode(previous.mode, next.mode) {
        publish_window_mode_changed(runtime_state, window, previous.mode, next.mode);
    }

    // publish display transitions
    if previous.display != next.display {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::DisplayChanged {
                window,
                previous_display: previous.display,
                current_display: next.display,
            }),
        );
    }

    // publish theme transitions
    if previous.theme != next.theme {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::ThemeChanged {
                window,
                previous_theme: previous.theme,
                current_theme: next.theme,
            }),
        );
    }

    // publish chrome transitions
    if previous.chrome != next.chrome {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::ChromeChanged {
                window,
                previous_chrome: previous.chrome,
                current_chrome: next.chrome,
            }),
        );
    }

    // publish taskbar-visibility transitions
    if previous.taskbar_visible != next.taskbar_visible {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::TaskbarVisibilityChanged {
                window,
                previous_taskbar_visible: previous.taskbar_visible,
                current_taskbar_visible: next.taskbar_visible,
            }),
        );
    }

    // publish safe-area transitions
    if previous.safe_area_insets != next.safe_area_insets {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::SafeAreaChanged {
                window,
                previous_safe_area_insets: previous.safe_area_insets,
                current_safe_area_insets: next.safe_area_insets,
            }),
        );
    }

    // publish opacity transitions
    if previous.opacity != next.opacity {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::OpacityChanged {
                window,
                previous_opacity: previous.opacity,
                current_opacity: next.opacity,
            }),
        );
    }

    // publish parent transitions
    if previous.parent != next.parent {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::ParentChanged {
                window,
                previous_parent: previous.parent,
                current_parent: next.parent,
            }),
        );
    }

    // publish transient-owner transitions
    if previous.transient_for != next.transient_for {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::TransientChanged {
                window,
                previous_transient_for: previous.transient_for,
                current_transient_for: next.transient_for,
            }),
        );
    }

    // publish modal transitions
    if previous.modal != next.modal {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::ModalChanged {
                window,
                previous_modal: previous.modal,
                current_modal: next.modal,
            }),
        );
    }

    // publish mouse-passthrough transitions
    if previous.mouse_passthrough != next.mouse_passthrough {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::MousePassthroughChanged {
                window,
                previous_mouse_passthrough: previous.mouse_passthrough,
                current_mouse_passthrough: next.mouse_passthrough,
            }),
        );
    }

    // publish aspect-ratio transitions
    if previous.aspect_ratio != next.aspect_ratio {
        publish_window_event(
            runtime_state,
            window_event_record(WindowEventRecordKind::AspectRatioChanged {
                window,
                previous_aspect_ratio: previous.aspect_ratio,
                current_aspect_ratio: next.aspect_ratio,
            }),
        );
    }
}
