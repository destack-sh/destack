use std::sync::Arc;

use objc2_app_kit::NSCursor;
use objc2_core_foundation::CGPoint;
use objc2_core_graphics::{CGError, CGWarpMouseCursorPosition};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowCursorIcon, WindowCursorMode, WindowPosition, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::{core as appkit_core, resource as display_resource};
use super::runtime;

/// Resolve one AppKit cursor instance for one runtime cursor icon.
#[allow(deprecated)]
fn cursor_from_icon(icon: WindowCursorIcon) -> objc2::rc::Retained<NSCursor> {
    match icon {
        WindowCursorIcon::Default | WindowCursorIcon::Pointer => NSCursor::arrowCursor(),
        WindowCursorIcon::Text => NSCursor::IBeamCursor(),
        WindowCursorIcon::Crosshair => NSCursor::crosshairCursor(),
        WindowCursorIcon::Grab | WindowCursorIcon::Move | WindowCursorIcon::AllScroll => {
            NSCursor::openHandCursor()
        }
        WindowCursorIcon::Grabbing => NSCursor::closedHandCursor(),
        WindowCursorIcon::EResize
        | WindowCursorIcon::WResize
        | WindowCursorIcon::EwResize
        | WindowCursorIcon::ColResize => NSCursor::resizeLeftRightCursor(),
        WindowCursorIcon::NResize
        | WindowCursorIcon::SResize
        | WindowCursorIcon::NsResize
        | WindowCursorIcon::RowResize => NSCursor::resizeUpDownCursor(),
        WindowCursorIcon::NeResize
        | WindowCursorIcon::NwResize
        | WindowCursorIcon::SeResize
        | WindowCursorIcon::SwResize
        | WindowCursorIcon::NeswResize
        | WindowCursorIcon::NwseResize
        | WindowCursorIcon::Help
        | WindowCursorIcon::Wait
        | WindowCursorIcon::Progress
        | WindowCursorIcon::NotAllowed
        | WindowCursorIcon::ZoomIn
        | WindowCursorIcon::ZoomOut => NSCursor::arrowCursor(),
    }
}

/// Resolve the effective process-global cursor policy for one AppKit runtime.
fn desired_cursor_policy(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
) -> (bool, WindowCursorIcon) {
    appkit_core::with_main_thread_state(runtime_state, |state| {
        let state = state.borrow();
        let mut is_hidden = false;
        let mut focused_icon = None;
        let mut fallback_icon = None;

        // inspect each live window binding once
        for host in state.windows.values() {
            let binding = host
                .binding
                .lock()
                .unwrap_or_else(|error| error.into_inner());

            // ignore fully hidden and minimized windows for cursor ownership
            if !matches!(binding.visibility, WindowVisibility::Visible) {
                continue;
            }

            // escalate to hidden when any visible window requests hidden policy
            if !binding.cursor_visible || binding.cursor_mode == WindowCursorMode::Hidden {
                is_hidden = true;
            }

            // prefer the focused window icon and fall back to the first visible window icon
            if binding.focused {
                focused_icon = Some(binding.cursor_icon);
            } else if fallback_icon.is_none() {
                fallback_icon = Some(binding.cursor_icon);
            }
        }

        (
            is_hidden,
            focused_icon
                .or(fallback_icon)
                .unwrap_or(WindowCursorIcon::Default),
        )
    })
}

/// Reconcile the process-global AppKit cursor state against runtime window policy.
pub(crate) fn apply_cursor_policy(runtime_state: &Arc<appkit_core::AppKitRuntimeState>) {
    let (is_hidden, icon) = desired_cursor_policy(runtime_state);
    let mut cursor_hidden = runtime_state
        .cursor_hidden
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // hide once when the runtime transitions into hidden cursor policy
    if is_hidden && !*cursor_hidden {
        NSCursor::hide();
        *cursor_hidden = true;
        return;
    }

    // restore once when the runtime transitions back to a visible cursor
    if !is_hidden && *cursor_hidden {
        NSCursor::unhide();
        *cursor_hidden = false;
    }

    // apply the focused cursor shape when the cursor is visible
    if !*cursor_hidden {
        let cursor = cursor_from_icon(icon);
        cursor.set();
    }
}

/// Set cursor icon for one window.
pub(crate) unsafe fn window_set_cursor_icon(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setCursorIcon",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setCursorIcon")?;
    binding.cursor_icon = icon;
    drop(binding);

    apply_cursor_policy(&runtime_state);
    Ok(())
}

/// Set cursor interaction mode for one window.
pub(crate) unsafe fn window_set_cursor_mode(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setCursorMode",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setCursorMode")?;

    // reject modes that AppKit does not expose as truthful low-level guarantees
    if matches!(mode, WindowCursorMode::Locked | WindowCursorMode::Confined) {
        return Err(core_platform::not_supported(
            "destack.display.window.setCursorMode",
        ));
    }

    binding.cursor_mode = mode;
    drop(binding);

    apply_cursor_policy(&runtime_state);
    Ok(())
}

/// Set cursor position for one window.
pub(crate) unsafe fn window_set_cursor_position(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.setCursorPosition",
        |host| {
            let screen_point = host
                .window
                .convertPointToScreen(objc2_foundation::NSPoint::new(
                    position.x as f64,
                    position.y as f64,
                ));
            let status = CGWarpMouseCursorPosition(CGPoint::new(screen_point.x, screen_point.y));

            // surface CoreGraphics cursor-warp failures explicitly
            if status != CGError(0) {
                return Err(appkit_core::io_error(
                    "destack.display.window.setCursorPosition",
                    format!("CGWarpMouseCursorPosition failed with status {:?}", status),
                ));
            }

            Ok(())
        },
    )?;

    Ok(())
}

/// Set cursor visibility for one window.
pub(crate) unsafe fn window_set_cursor_visible(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setCursorVisible",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.setCursorVisible")?;
    binding.cursor_visible = visible;
    drop(binding);

    apply_cursor_policy(&runtime_state);
    Ok(())
}
