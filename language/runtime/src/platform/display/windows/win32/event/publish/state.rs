use std::sync::Arc;

use crate::platform::display::windows::win32::core::Win32RuntimeState;
use crate::platform::display::windows::win32::model::Win32WindowHostState;
use crate::platform::display::windows::win32::window;
use crate::platform::display::{
    WindowAspectRatio, WindowChromeKind, WindowSafeAreaInsets, WindowTheme,
};
use crate::platform::resource;

use super::core::{
    publish_window_mode_changed, publish_window_position_changed,
    publish_window_scale_factor_changed, publish_window_size_changed,
    publish_window_visibility_changed,
};
use crate::platform::display::windows::win32::event::queue::publish_window_event;
use crate::platform::display::windows::win32::event::{WindowEventRecordKind, window_event_record};

/// Publish one theme-changed window event.
fn publish_window_theme_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_theme: WindowTheme,
    current_theme: WindowTheme,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::ThemeChanged {
            window,
            previous_theme,
            current_theme,
        }),
    );
}

/// Publish one chrome-changed window event.
fn publish_window_chrome_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_chrome: WindowChromeKind,
    current_chrome: WindowChromeKind,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::ChromeChanged {
            window,
            previous_chrome,
            current_chrome,
        }),
    );
}

/// Publish one taskbar-visibility-changed window event.
fn publish_window_taskbar_visibility_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_taskbar_visible: bool,
    current_taskbar_visible: bool,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::TaskbarVisibilityChanged {
            window,
            previous_taskbar_visible,
            current_taskbar_visible,
        }),
    );
}

/// Publish one opacity-changed window event.
fn publish_window_opacity_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_opacity: f64,
    current_opacity: f64,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::OpacityChanged {
            window,
            previous_opacity,
            current_opacity,
        }),
    );
}

/// Publish one safe-area-changed window event.
fn publish_window_safe_area_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_safe_area_insets: Option<WindowSafeAreaInsets>,
    current_safe_area_insets: Option<WindowSafeAreaInsets>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::SafeAreaChanged {
            window,
            previous_safe_area_insets,
            current_safe_area_insets,
        }),
    );
}

/// Publish one parent-changed window event.
fn publish_window_parent_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_parent: Option<resource::WindowHandle>,
    current_parent: Option<resource::WindowHandle>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::ParentChanged {
            window,
            previous_parent,
            current_parent,
        }),
    );
}

/// Publish one display-changed window event.
pub(crate) fn publish_window_display_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_display: Option<resource::DisplayHandle>,
    current_display: Option<resource::DisplayHandle>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::DisplayChanged {
            window,
            previous_display,
            current_display,
        }),
    );
}

/// Publish one transient-owner-changed window event.
fn publish_window_transient_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_transient_for: Option<resource::WindowHandle>,
    current_transient_for: Option<resource::WindowHandle>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::TransientChanged {
            window,
            previous_transient_for,
            current_transient_for,
        }),
    );
}

/// Publish one modal-changed window event.
fn publish_window_modal_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_modal: bool,
    current_modal: bool,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::ModalChanged {
            window,
            previous_modal,
            current_modal,
        }),
    );
}

/// Publish one mouse-passthrough-changed window event.
fn publish_window_mouse_passthrough_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_mouse_passthrough: bool,
    current_mouse_passthrough: bool,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::MousePassthroughChanged {
            window,
            previous_mouse_passthrough,
            current_mouse_passthrough,
        }),
    );
}

/// Publish one aspect-ratio-changed window event.
fn publish_window_aspect_ratio_changed(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous_aspect_ratio: Option<WindowAspectRatio>,
    current_aspect_ratio: Option<WindowAspectRatio>,
) {
    publish_window_event(
        runtime_state,
        window_event_record(WindowEventRecordKind::AspectRatioChanged {
            window,
            previous_aspect_ratio,
            current_aspect_ratio,
        }),
    );
}

/// Publish one focus-changed window event.
pub(crate) fn publish_window_focus_changed(
    runtime_state: &Arc<Win32RuntimeState>,
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

/// Publish all state transitions observed between two window snapshots.
pub(crate) fn publish_state_deltas(
    runtime_state: &Arc<Win32RuntimeState>,
    window: resource::WindowHandle,
    previous: &Win32WindowHostState,
    next: &Win32WindowHostState,
) {
    // visibility and derived occlusion
    if previous.visibility != next.visibility {
        publish_window_visibility_changed(
            runtime_state,
            window,
            previous.visibility,
            next.visibility,
        );
    }

    // geometry and scale
    if previous.position != next.position {
        publish_window_position_changed(runtime_state, window, previous.position, next.position);
    }

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

    if previous.scale_factor_milli != next.scale_factor_milli {
        publish_window_scale_factor_changed(
            runtime_state,
            window,
            previous.scale_factor_milli,
            next.scale_factor_milli,
        );
    }

    // focus and presentation
    if previous.focused != next.focused {
        publish_window_focus_changed(runtime_state, window, previous.focused, next.focused);
    }

    if previous.theme != next.theme {
        publish_window_theme_changed(runtime_state, window, previous.theme, next.theme);
    }

    if previous.chrome != next.chrome {
        publish_window_chrome_changed(runtime_state, window, previous.chrome, next.chrome);
    }

    if previous.taskbar_visible != next.taskbar_visible {
        publish_window_taskbar_visibility_changed(
            runtime_state,
            window,
            previous.taskbar_visible,
            next.taskbar_visible,
        );
    }

    if previous.safe_area_insets != next.safe_area_insets {
        publish_window_safe_area_changed(
            runtime_state,
            window,
            previous.safe_area_insets,
            next.safe_area_insets,
        );
    }

    if previous.opacity != next.opacity {
        publish_window_opacity_changed(runtime_state, window, previous.opacity, next.opacity);
    }

    // relationships and policy
    if previous.parent != next.parent {
        publish_window_parent_changed(runtime_state, window, previous.parent, next.parent);
    }

    if previous.transient_for != next.transient_for {
        publish_window_transient_changed(
            runtime_state,
            window,
            previous.transient_for,
            next.transient_for,
        );
    }

    if previous.modal != next.modal {
        publish_window_modal_changed(runtime_state, window, previous.modal, next.modal);
    }

    if previous.mouse_passthrough != next.mouse_passthrough {
        publish_window_mouse_passthrough_changed(
            runtime_state,
            window,
            previous.mouse_passthrough,
            next.mouse_passthrough,
        );
    }

    if previous.aspect_ratio != next.aspect_ratio {
        publish_window_aspect_ratio_changed(
            runtime_state,
            window,
            previous.aspect_ratio,
            next.aspect_ratio,
        );
    }

    if !window::same_window_mode(previous.mode, next.mode) {
        publish_window_mode_changed(runtime_state, window, previous.mode, next.mode);
    }

    if previous.display != next.display {
        publish_window_display_changed(runtime_state, window, previous.display, next.display);
    }
}
