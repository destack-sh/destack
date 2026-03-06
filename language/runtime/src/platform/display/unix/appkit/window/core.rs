use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAppearanceNameAccessibilityHighContrastAqua,
    NSAppearanceNameAccessibilityHighContrastDarkAqua,
    NSAppearanceNameAccessibilityHighContrastVibrantDark,
    NSAppearanceNameAccessibilityHighContrastVibrantLight, NSAppearanceNameDarkAqua,
    NSAppearanceNameVibrantDark, NSApplication, NSScreen, NSWindow, NSWindowOcclusionState,
    NSWindowStyleMask,
};
use objc2_foundation::{NSNumber, ns_string};

use crate::platform::display::{
    WindowDescriptor, WindowLogicalSize, WindowOcclusionState, WindowPhysicalSize, WindowPosition,
    WindowSafeAreaInsets, WindowState, WindowTheme, WindowVisibility,
};
use crate::runtime::BindingCallContext;

use super::super::model::AppKitWindowBinding;
use super::super::{core as appkit_core, monitor};

/// Resolve one occlusion value from one native AppKit window.
pub(crate) fn occlusion_from_window(window: &NSWindow) -> WindowOcclusionState {
    let state = window.occlusionState();

    // treat AppKit-visible windows as unoccluded
    if state.contains(NSWindowOcclusionState::Visible) {
        return WindowOcclusionState::Unoccluded;
    }

    WindowOcclusionState::Occluded
}

/// Build one descriptor payload from one binding snapshot.
pub(crate) fn descriptor_from_binding(
    context: &BindingCallContext,
    binding: &AppKitWindowBinding,
) -> WindowDescriptor {
    WindowDescriptor {
        backend: appkit_core::selected_backend(),
        id: context.store_string(&binding.id),
        title: context.store_string(&binding.title),
        role: binding.role,
        mode: binding.mode,
        display: binding.display,
        resizable: binding.resizable,
        decorated: binding.decorated,
        chrome: binding.chrome,
        taskbar_visible: binding.taskbar_visible,
        transparent: binding.transparent,
        opacity: binding.opacity,
        always_on_top: binding.always_on_top,
        parent: binding.parent,
        transient_for: binding.transient_for,
        modal: binding.modal,
        mouse_passthrough: binding.mouse_passthrough,
        aspect_ratio: binding.aspect_ratio,
    }
}

/// Build one window state payload from one binding snapshot.
pub(crate) fn state_from_binding(binding: &AppKitWindowBinding) -> WindowState {
    WindowState {
        backend: appkit_core::selected_backend(),
        position: binding.position,
        size_logical: binding.size_logical,
        size_physical: binding.size_physical,
        scale_factor_milli: binding.scale_factor_milli,
        visibility: binding.visibility,
        role: binding.role,
        display: binding.display,
        focused: binding.focused,
        occlusion: binding.occlusion,
        safe_area_insets: binding.safe_area_insets,
        theme: binding.theme,
        chrome: binding.chrome,
        taskbar_visible: binding.taskbar_visible,
        opacity: binding.opacity,
        always_on_top: binding.always_on_top,
        parent: binding.parent,
        transient_for: binding.transient_for,
        modal: binding.modal,
        mouse_passthrough: binding.mouse_passthrough,
        aspect_ratio: binding.aspect_ratio,
    }
}

/// Resolve one theme value from the current AppKit appearance.
fn theme_from_application() -> WindowTheme {
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

/// Refresh one binding geometry snapshot from one native AppKit window.
pub(crate) fn refresh_binding_geometry(binding: &mut AppKitWindowBinding, window: &NSWindow) {
    let frame = window.frame();
    let content_rect = window.contentRectForFrameRect(frame);
    let scale_factor_milli = (window.backingScaleFactor() * 1000.0).round() as u32;
    let width = content_rect.size.width.max(1.0);
    let height = content_rect.size.height.max(1.0);

    binding.position = WindowPosition {
        x: frame.origin.x.round() as i32,
        y: frame.origin.y.round() as i32,
    };
    binding.size_logical = WindowLogicalSize { width, height };
    binding.size_physical = WindowPhysicalSize {
        width: (width * window.backingScaleFactor()).round().max(1.0) as u32,
        height: (height * window.backingScaleFactor()).round().max(1.0) as u32,
    };
    binding.scale_factor_milli = scale_factor_milli.max(1);
    binding.focused = window.isKeyWindow();
    binding.visibility = if window.isMiniaturized() {
        WindowVisibility::Minimized
    } else if !window.isVisible() {
        WindowVisibility::Hidden
    } else if window.styleMask().contains(NSWindowStyleMask::FullScreen) || window.isZoomed() {
        WindowVisibility::Maximized
    } else {
        WindowVisibility::Visible
    };
    binding.occlusion = occlusion_from_window(window);
    binding.safe_area_insets = safe_area_insets_from_window(window);
    binding.theme = theme_from_application();
}
