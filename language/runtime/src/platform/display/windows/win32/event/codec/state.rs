use crate::platform::display::windows::win32::event::{WindowEventRecord, WindowEventRecordKind};
use crate::platform::display::{
    WindowAspectRatioChangedEvent, WindowAspectRatioPayload, WindowChromeChangedEvent,
    WindowChromePayload, WindowCloseRequestedEvent, WindowCreatedEvent, WindowDestroyedEvent,
    WindowDisplayChangedEvent, WindowDisplayPayload, WindowEvent, WindowFocusChangedEvent,
    WindowFocusPayload, WindowModalChangedEvent, WindowModalPayload, WindowModeChangedEvent,
    WindowModePayload, WindowMousePassthroughChangedEvent, WindowMousePassthroughPayload,
    WindowOpacityChangedEvent, WindowOpacityPayload, WindowParentChangedEvent, WindowParentPayload,
    WindowPositionChangedEvent, WindowPositionPayload, WindowSafeAreaChangedEvent,
    WindowSafeAreaPayload, WindowScaleFactorChangedEvent, WindowScaleFactorPayload,
    WindowSizeChangedEvent, WindowSizePayload, WindowTaskbarVisibilityChangedEvent,
    WindowTaskbarVisibilityPayload, WindowThemeChangedEvent, WindowThemePayload,
    WindowTransientChangedEvent, WindowTransientPayload, WindowVisibilityChangedEvent,
    WindowVisibilityPayload,
};
use crate::runtime::BindingCallContext;

use super::core::window_event_metadata;

/// Convert one non-drop window-event record into one ABI event payload.
pub(crate) fn window_state_event_from_record(
    value: WindowEventRecord,
    binding: &BindingCallContext,
) -> WindowEvent {
    // decode this state-event variant
    match value.kind {
        WindowEventRecordKind::Created { window } => {
            WindowEvent::WindowCreatedEvent(WindowCreatedEvent {
                kind: binding.store_string("created"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::CloseRequested { window } => {
            WindowEvent::WindowCloseRequestedEvent(WindowCloseRequestedEvent {
                kind: binding.store_string("closeRequested"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::Destroyed { window } => {
            WindowEvent::WindowDestroyedEvent(WindowDestroyedEvent {
                kind: binding.store_string("destroyed"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
            })
        }
        WindowEventRecordKind::FocusChanged {
            window,
            previous_focused,
            current_focused,
        } => WindowEvent::WindowFocusChangedEvent(WindowFocusChangedEvent {
            kind: binding.store_string("focusChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowFocusPayload {
                previous_focused,
                current_focused,
            },
        }),
        WindowEventRecordKind::VisibilityChanged {
            window,
            previous_visibility,
            current_visibility,
        } => WindowEvent::WindowVisibilityChangedEvent(WindowVisibilityChangedEvent {
            kind: binding.store_string("visibilityChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowVisibilityPayload {
                previous_visibility,
                current_visibility,
            },
        }),
        WindowEventRecordKind::PositionChanged {
            window,
            previous_position,
            current_position,
        } => WindowEvent::WindowPositionChangedEvent(WindowPositionChangedEvent {
            kind: binding.store_string("positionChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowPositionPayload {
                previous_position,
                current_position,
            },
        }),
        WindowEventRecordKind::SizeChanged {
            window,
            previous_size_logical,
            previous_size_physical,
            current_size_logical,
            current_size_physical,
        } => WindowEvent::WindowSizeChangedEvent(WindowSizeChangedEvent {
            kind: binding.store_string("sizeChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowSizePayload {
                previous_size_logical,
                previous_size_physical,
                current_size_logical,
                current_size_physical,
            },
        }),
        WindowEventRecordKind::ScaleFactorChanged {
            window,
            previous_scale_factor_milli,
            current_scale_factor_milli,
        } => WindowEvent::WindowScaleFactorChangedEvent(WindowScaleFactorChangedEvent {
            kind: binding.store_string("scaleFactorChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowScaleFactorPayload {
                previous_scale_factor_milli,
                current_scale_factor_milli,
            },
        }),
        WindowEventRecordKind::RefreshRequested { .. } => {
            unreachable!("refreshRequested window events must route through begin-frame")
        }
        WindowEventRecordKind::ModeChanged {
            window,
            previous_mode,
            current_mode,
        } => WindowEvent::WindowModeChangedEvent(WindowModeChangedEvent {
            kind: binding.store_string("modeChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowModePayload {
                previous_mode,
                current_mode,
            },
        }),
        WindowEventRecordKind::DisplayChanged {
            window,
            previous_display,
            current_display,
        } => WindowEvent::WindowDisplayChangedEvent(WindowDisplayChangedEvent {
            kind: binding.store_string("displayChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowDisplayPayload {
                previous_display,
                current_display,
            },
        }),
        WindowEventRecordKind::ThemeChanged {
            window,
            previous_theme,
            current_theme,
        } => WindowEvent::WindowThemeChangedEvent(WindowThemeChangedEvent {
            kind: binding.store_string("themeChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowThemePayload {
                previous_theme,
                current_theme,
            },
        }),
        WindowEventRecordKind::ChromeChanged {
            window,
            previous_chrome,
            current_chrome,
        } => WindowEvent::WindowChromeChangedEvent(WindowChromeChangedEvent {
            kind: binding.store_string("chromeChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowChromePayload {
                previous_chrome,
                current_chrome,
            },
        }),
        WindowEventRecordKind::TaskbarVisibilityChanged {
            window,
            previous_taskbar_visible,
            current_taskbar_visible,
        } => {
            WindowEvent::WindowTaskbarVisibilityChangedEvent(WindowTaskbarVisibilityChangedEvent {
                kind: binding.store_string("taskbarVisibilityChanged"),
                metadata: window_event_metadata(
                    window,
                    value.timestamp_ns,
                    value.sequence,
                    value.dropped_count,
                ),
                payload: WindowTaskbarVisibilityPayload {
                    previous_taskbar_visible,
                    current_taskbar_visible,
                },
            })
        }
        WindowEventRecordKind::SafeAreaChanged {
            window,
            previous_safe_area_insets,
            current_safe_area_insets,
        } => WindowEvent::WindowSafeAreaChangedEvent(WindowSafeAreaChangedEvent {
            kind: binding.store_string("safeAreaChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowSafeAreaPayload {
                previous_safe_area_insets,
                current_safe_area_insets,
            },
        }),
        WindowEventRecordKind::OpacityChanged {
            window,
            previous_opacity,
            current_opacity,
        } => WindowEvent::WindowOpacityChangedEvent(WindowOpacityChangedEvent {
            kind: binding.store_string("opacityChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowOpacityPayload {
                previous_opacity,
                current_opacity,
            },
        }),
        WindowEventRecordKind::ParentChanged {
            window,
            previous_parent,
            current_parent,
        } => WindowEvent::WindowParentChangedEvent(WindowParentChangedEvent {
            kind: binding.store_string("parentChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowParentPayload {
                previous_parent,
                current_parent,
            },
        }),
        WindowEventRecordKind::TransientChanged {
            window,
            previous_transient_for,
            current_transient_for,
        } => WindowEvent::WindowTransientChangedEvent(WindowTransientChangedEvent {
            kind: binding.store_string("transientChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowTransientPayload {
                previous_transient_for,
                current_transient_for,
            },
        }),
        WindowEventRecordKind::ModalChanged {
            window,
            previous_modal,
            current_modal,
        } => WindowEvent::WindowModalChangedEvent(WindowModalChangedEvent {
            kind: binding.store_string("modalChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowModalPayload {
                previous_modal,
                current_modal,
            },
        }),
        WindowEventRecordKind::MousePassthroughChanged {
            window,
            previous_mouse_passthrough,
            current_mouse_passthrough,
        } => WindowEvent::WindowMousePassthroughChangedEvent(WindowMousePassthroughChangedEvent {
            kind: binding.store_string("mousePassthroughChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowMousePassthroughPayload {
                previous_mouse_passthrough,
                current_mouse_passthrough,
            },
        }),
        WindowEventRecordKind::AspectRatioChanged {
            window,
            previous_aspect_ratio,
            current_aspect_ratio,
        } => WindowEvent::WindowAspectRatioChangedEvent(WindowAspectRatioChangedEvent {
            kind: binding.store_string("aspectRatioChanged"),
            metadata: window_event_metadata(
                window,
                value.timestamp_ns,
                value.sequence,
                value.dropped_count,
            ),
            payload: WindowAspectRatioPayload {
                previous_aspect_ratio,
                current_aspect_ratio,
            },
        }),

        // drop events are routed through the drop codec
        WindowEventRecordKind::DropStarted { .. }
        | WindowEventRecordKind::FileHovered { .. }
        | WindowEventRecordKind::DropCancelled { .. }
        | WindowEventRecordKind::DropCompleted { .. }
        | WindowEventRecordKind::FileHoverLeft { .. }
        | WindowEventRecordKind::FileDropped { .. }
        | WindowEventRecordKind::TextDropped { .. } => unreachable!(),
    }
}
