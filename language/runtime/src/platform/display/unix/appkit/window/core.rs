use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAppearanceNameAccessibilityHighContrastAqua,
    NSAppearanceNameAccessibilityHighContrastDarkAqua,
    NSAppearanceNameAccessibilityHighContrastVibrantDark,
    NSAppearanceNameAccessibilityHighContrastVibrantLight, NSAppearanceNameDarkAqua,
    NSAppearanceNameVibrantDark, NSApplication, NSScreen, NSWindow, NSWindowOcclusionState,
    NSWindowStyleMask,
};
use objc2_core_graphics::{CGDisplayBounds, CGMainDisplayID};
use objc2_foundation::{NSNumber, ns_string};

use crate::platform::display::{
    WindowDescriptor, WindowLogicalSize, WindowOcclusionState, WindowPhysicalSize, WindowPosition,
    WindowSafeAreaInsets, WindowState, WindowTheme, WindowVisibility,
};
use crate::runtime::BindingCallContext;

use crate::platform::display::unix::appkit::model::AppKitWindowHostState;
use crate::platform::display::unix::appkit::{core as appkit_core, monitor};

/// Resolve one occlusion value from one native AppKit window.
pub(crate) fn occlusion_from_window(window: &NSWindow) -> WindowOcclusionState {
    let state = window.occlusionState();

    // treat AppKit-visible windows as unoccluded
    if state.contains(NSWindowOcclusionState::Visible) {
        return WindowOcclusionState::Unoccluded;
    }

    WindowOcclusionState::Occluded
}

/// Build one descriptor payload from one host-state snapshot.
pub(crate) fn descriptor_from_host_state(
    context: &BindingCallContext,
    host_state: &AppKitWindowHostState,
) -> WindowDescriptor {
    WindowDescriptor {
        backend: appkit_core::selected_backend(),
        id: context.store_string(&host_state.id),
        title: context.store_string(&host_state.title),
        role: host_state.role,
        mode: host_state.mode,
        display: host_state.display,
        resizable: host_state.resizable,
        decorated: host_state.decorated,
        chrome: host_state.chrome,
        taskbar_visible: host_state.taskbar_visible,
        transparent: host_state.transparent,
        opacity: host_state.opacity,
        always_on_top: host_state.always_on_top,
        parent: host_state.parent,
        transient_for: host_state.transient_for,
        modal: host_state.modal,
        mouse_passthrough: host_state.mouse_passthrough,
        aspect_ratio: host_state.aspect_ratio,
    }
}

/// Build one window state payload from one host-state snapshot.
pub(crate) fn state_from_host_state(host_state: &AppKitWindowHostState) -> WindowState {
    WindowState {
        backend: appkit_core::selected_backend(),
        position: host_state.position,
        size_logical: host_state.size_logical,
        size_physical: host_state.size_physical,
        scale_factor_milli: host_state.scale_factor_milli,
        visibility: host_state.visibility,
        role: host_state.role,
        display: host_state.display,
        focused: host_state.focused,
        occlusion: host_state.occlusion,
        safe_area_insets: host_state.safe_area_insets,
        theme: host_state.theme,
        chrome: host_state.chrome,
        taskbar_visible: host_state.taskbar_visible,
        opacity: host_state.opacity,
        always_on_top: host_state.always_on_top,
        parent: host_state.parent,
        transient_for: host_state.transient_for,
        modal: host_state.modal,
        mouse_passthrough: host_state.mouse_passthrough,
        aspect_ratio: host_state.aspect_ratio,
    }
}

/// Resolve one theme value from the current AppKit appearance.
pub(crate) fn current_window_theme() -> WindowTheme {
    let Some(mtm) = MainThreadMarker::new() else {
        return WindowTheme::Unknown;
    };
    let application = NSApplication::sharedApplication(mtm);
    let appearance = application.effectiveAppearance();
    let appearance_name = appearance.name();
    let appearance_name = appearance_name.to_string();
    let high_contrast_dark =
        unsafe { NSAppearanceNameAccessibilityHighContrastDarkAqua }.to_string();
    let high_contrast_vibrant_dark =
        unsafe { NSAppearanceNameAccessibilityHighContrastVibrantDark }.to_string();
    let high_contrast_light = unsafe { NSAppearanceNameAccessibilityHighContrastAqua }.to_string();
    let high_contrast_vibrant_light =
        unsafe { NSAppearanceNameAccessibilityHighContrastVibrantLight }.to_string();
    let dark = unsafe { NSAppearanceNameDarkAqua }.to_string();
    let vibrant_dark = unsafe { NSAppearanceNameVibrantDark }.to_string();

    // map high-contrast dark appearances first
    if appearance_name == high_contrast_dark || appearance_name == high_contrast_vibrant_dark {
        return WindowTheme::HighContrastDark;
    }

    // map high-contrast light appearances next
    if appearance_name == high_contrast_light || appearance_name == high_contrast_vibrant_light {
        return WindowTheme::HighContrastLight;
    }

    // map dark appearances before falling back to light
    if appearance_name == dark || appearance_name == vibrant_dark {
        return WindowTheme::Dark;
    }

    WindowTheme::Light
}

/// Resolve one safe-area inset payload from one native AppKit window.
fn safe_area_insets_from_window(window: &NSWindow) -> Option<WindowSafeAreaInsets> {
    let content_view = window.contentView()?;
    let insets = content_view.safeAreaInsets();
    let scale = window.backingScaleFactor();

    Some(WindowSafeAreaInsets {
        left_px: (insets.left * scale).round().max(0.0) as u32,
        top_px: (insets.top * scale).round().max(0.0) as u32,
        right_px: (insets.right * scale).round().max(0.0) as u32,
        bottom_px: (insets.bottom * scale).round().max(0.0) as u32,
    })
}

/// Resolve one stable display id from one native AppKit screen.
pub(crate) fn display_id_from_screen(screen: &NSScreen) -> Option<String> {
    let description = screen.deviceDescription();
    let display_number = description.objectForKey(ns_string!("NSScreenNumber"))?;

    let display_number = display_number.downcast_ref::<NSNumber>()?;
    let display_number = display_number.as_u32();

    Some(monitor::display_id(display_number))
}

/// Return the main-display desktop height used by AppKit screen coordinates.
fn main_display_height() -> f64 {
    CGDisplayBounds(CGMainDisplayID()).size.height
}

/// Convert one runtime desktop position into one AppKit frame origin.
pub(crate) fn frame_origin_from_desktop_position(
    position: WindowPosition,
    frame_height: f64,
) -> objc2_foundation::NSPoint {
    let main_display_height = main_display_height();
    let origin_y = main_display_height - frame_height - (position.y as f64);

    objc2_foundation::NSPoint::new(position.x as f64, origin_y)
}

/// Convert one runtime desktop position into one AppKit frame top-left point.
pub(crate) fn frame_top_left_point_from_desktop_position(
    position: WindowPosition,
) -> objc2_foundation::NSPoint {
    let main_display_height = main_display_height();
    let top_left_y = main_display_height - (position.y as f64);

    objc2_foundation::NSPoint::new(position.x as f64, top_left_y)
}

/// Convert one AppKit frame rect into one runtime desktop position.
pub(crate) fn desktop_position_from_frame(frame: objc2_foundation::NSRect) -> WindowPosition {
    let main_display_height = main_display_height();
    let position_y = main_display_height - frame.size.height - frame.origin.y;

    WindowPosition {
        x: frame.origin.x.round() as i32,
        y: position_y.round() as i32,
    }
}

/// Refresh one host-state geometry snapshot from one native AppKit window.
pub(crate) fn refresh_host_state_geometry(
    host_state: &mut AppKitWindowHostState,
    window: &NSWindow,
) {
    if window.screen().is_none() {
        host_state.focused = window.isKeyWindow();
        host_state.visibility = if window.isMiniaturized() {
            WindowVisibility::Minimized
        } else if !window.isVisible() {
            WindowVisibility::Hidden
        } else if window.styleMask().contains(NSWindowStyleMask::FullScreen) || window.isZoomed() {
            WindowVisibility::Maximized
        } else {
            WindowVisibility::Visible
        };
        host_state.occlusion = occlusion_from_window(window);
        host_state.safe_area_insets = safe_area_insets_from_window(window);
        host_state.theme = current_window_theme();
        return;
    }

    let frame = window.frame();
    let content_rect = window.contentRectForFrameRect(frame);
    let scale_factor_milli = (window.backingScaleFactor() * 1000.0).round() as u32;
    let width = content_rect.size.width.max(1.0);
    let height = content_rect.size.height.max(1.0);

    host_state.position = desktop_position_from_frame(frame);
    host_state.size_logical = WindowLogicalSize { width, height };
    host_state.size_physical = WindowPhysicalSize {
        width: (width * window.backingScaleFactor()).round().max(1.0) as u32,
        height: (height * window.backingScaleFactor()).round().max(1.0) as u32,
    };
    host_state.scale_factor_milli = scale_factor_milli.max(1);
    host_state.focused = window.isKeyWindow();
    host_state.visibility = if window.isMiniaturized() {
        WindowVisibility::Minimized
    } else if !window.isVisible() {
        WindowVisibility::Hidden
    } else if window.styleMask().contains(NSWindowStyleMask::FullScreen) || window.isZoomed() {
        WindowVisibility::Maximized
    } else {
        WindowVisibility::Visible
    };
    host_state.occlusion = occlusion_from_window(window);
    host_state.safe_area_insets = safe_area_insets_from_window(window);
    host_state.theme = current_window_theme();
}
