use objc2_app_kit::NSWindowStyleMask;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{
    WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowLogicalSize, WindowModeOptions,
    WindowOcclusionState, WindowOptions, WindowPhysicalSize, WindowPosition, WindowRole,
    WindowTheme, WindowVisibility,
};

use super::super::core as appkit_core;
use super::super::model::AppKitWindowBinding;
use super::constants::DEFAULT_WINDOW_OPACITY;
use super::mode;

/// Resolve role-specific defaults for one AppKit open request.
pub(crate) fn resolve_role_open_defaults(
    role: WindowRole,
    chrome: WindowChromeKind,
    decorated: bool,
    taskbar_visible: bool,
    always_on_top: bool,
) -> (WindowChromeKind, bool, bool, bool) {
    // keep explicit caller values for top-level windows
    if role == WindowRole::Toplevel {
        return (chrome, decorated, taskbar_visible, always_on_top);
    }

    // popup role defaults to popup chrome and hidden task-switcher presence
    if role == WindowRole::Popup {
        let chrome = if chrome == WindowChromeKind::Standard {
            WindowChromeKind::Popup
        } else {
            chrome
        };

        return (chrome, decorated, false, always_on_top);
    }

    // overlay role defaults to popup chrome, hidden switching presence, and topmost layering
    (WindowChromeKind::Popup, false, false, true)
}

/// Resolve one native window style mask from one window-open request.
pub(crate) fn style_mask_for_options(options: &WindowOptions) -> NSWindowStyleMask {
    let mut mask = if options.decorated {
        NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable
            | NSWindowStyleMask::Resizable
    } else {
        NSWindowStyleMask::Borderless
    };

    // clear the resizable style bit when the caller disables resizing
    if !options.resizable {
        mask.remove(NSWindowStyleMask::Resizable);
    }

    // mark popup roles as utility windows
    if options.role == WindowRole::Popup {
        mask.insert(NSWindowStyleMask::UtilityWindow);
    }

    mask
}

/// Validate unsupported open-option lanes for the AppKit backend.
pub(crate) fn validate_unsupported_open_options(options: WindowOptions) -> RuntimeResult<()> {
    // reject unsupported exclusive fullscreen requests
    if matches!(
        options.mode,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
    ) {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // validate popup ownership before reaching AppKit creation
    if options.role == WindowRole::Popup
        && options.parent.is_none()
        && options.transient_for.is_none()
    {
        return Err(core_platform::invalid_argument(
            "options.role",
            "popup windows require parent or transientFor relationship",
        ));
    }

    // validate modal ownership before reporting backend support
    if options.modal.unwrap_or(false) && options.parent.is_none() && options.transient_for.is_none()
    {
        return Err(core_platform::invalid_argument(
            "options.modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // reject modal requests for non-toplevel roles
    if options.modal.unwrap_or(false) && options.role != WindowRole::Toplevel {
        return Err(core_platform::invalid_argument(
            "options.modal",
            "modal windows require the toplevel role on AppKit",
        ));
    }

    // reject explicit task-switcher hiding for standard top-level windows
    if !options.taskbar_visible && options.role == WindowRole::Toplevel {
        return Err(core_platform::not_supported("destack.display.window.open"));
    }

    // validate aspect-ratio payload shape
    if let Some(aspect_ratio) = options.aspect_ratio
        && (aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0)
    {
        return Err(core_platform::invalid_argument(
            "options.aspectRatio",
            "aspect ratio numerator and denominator must be greater than zero",
        ));
    }

    // reject conflicting explicit display selections
    if let (Some(display), Some(mode_display)) = (options.display, mode::mode_display(options.mode))
        && display != mode_display
    {
        return Err(core_platform::invalid_argument(
            "options",
            "options.display must match options.mode display when both are set",
        ));
    }

    Ok(())
}

/// Build one native window rect from one logical size and optional position.
pub(crate) fn window_rect(
    size: WindowLogicalSize,
    position: Option<WindowPosition>,
) -> objc2_foundation::NSRect {
    let origin = position.unwrap_or(WindowPosition { x: 0, y: 0 });

    objc2_foundation::NSRect::new(
        objc2_foundation::NSPoint::new(origin.x as f64, origin.y as f64),
        objc2_foundation::NSSize::new(size.width.max(1.0), size.height.max(1.0)),
    )
}

/// Build one initial binding snapshot for one open request.
pub(crate) fn initial_binding(options: &WindowOptions, title: &str) -> AppKitWindowBinding {
    let position = options.position.unwrap_or(WindowPosition { x: 0, y: 0 });
    let size_logical = options.size_logical;
    let size_physical = WindowPhysicalSize {
        width: size_logical.width.round().max(1.0) as u32,
        height: size_logical.height.round().max(1.0) as u32,
    };

    AppKitWindowBinding {
        id: format!(
            "{}-window-{}",
            appkit_core::selected_backend_name(),
            core_platform::monotonic_now_ns()
        ),
        owner_thread_id: std::thread::current().id(),
        title: title.to_string(),
        role: options.role,
        mode: options.mode,
        display: options.display,
        resizable: options.resizable,
        decorated: options.decorated,
        chrome: options.chrome,
        taskbar_visible: options.taskbar_visible,
        transparent: options.transparent,
        opacity: options.opacity.unwrap_or(DEFAULT_WINDOW_OPACITY),
        always_on_top: options.always_on_top,
        parent: options.parent,
        transient_for: options.transient_for,
        modal: options.modal.unwrap_or(false),
        mouse_passthrough: options.mouse_passthrough.unwrap_or(false),
        aspect_ratio: options.aspect_ratio,
        visibility: options.visibility,
        requested_visibility: Some(options.visibility),
        restored_visibility: if options.visibility == WindowVisibility::Maximized {
            WindowVisibility::Maximized
        } else {
            WindowVisibility::Visible
        },
        constraints: options.constraints,
        cursor_visible: true,
        cursor_mode: WindowCursorMode::Normal,
        cursor_icon: WindowCursorIcon::Default,
        position,
        size_logical,
        size_physical,
        scale_factor_milli: 1000,
        focused: false,
        occlusion: WindowOcclusionState::Unknown,
        safe_area_insets: None,
        theme: WindowTheme::Unknown,
    }
}
