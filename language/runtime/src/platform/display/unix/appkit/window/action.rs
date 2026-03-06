use dispatch2::run_on_main;
use objc2_app_kit::{NSApplication, NSRequestUserAttentionType};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowAttentionLevel, WindowResizeEdge, unsupported};
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::super::{core as appkit_core, event, resource as display_resource};
use super::runtime;

/// Validate one window action handle and owner thread, then return runtime state.
fn validated_runtime_state(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    operation: &'static str,
) -> RuntimeResult<std::sync::Arc<appkit_core::AppKitRuntimeState>> {
    let runtime_state = appkit_core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(context, window_handle, operation)?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    runtime::ensure_window_thread(&binding, operation)?;

    Ok(runtime_state)
}

/// Request one user-attention pulse for one window.
pub(crate) unsafe fn window_request_attention(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    let _runtime_state = validated_runtime_state(
        context,
        window_handle,
        "destack.display.window.requestAttention",
    )?;
    let request_type = if matches!(level, WindowAttentionLevel::Critical) {
        NSRequestUserAttentionType::CriticalRequest
    } else {
        NSRequestUserAttentionType::InformationalRequest
    };

    run_on_main(|mtm| {
        let application = NSApplication::sharedApplication(mtm);
        application.requestUserAttention(request_type);
    });

    Ok(())
}

/// Request one redraw for one window.
pub(crate) unsafe fn window_request_refresh(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    let runtime_state = validated_runtime_state(
        context,
        window_handle,
        "destack.display.window.requestRefresh",
    )?;

    event::publish_window_refresh_requested(&runtime_state, window_handle);

    Ok(())
}

/// Focus one window.
pub(crate) unsafe fn window_focus(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    let runtime_state =
        validated_runtime_state(context, window_handle, "destack.display.window.focus")?;
    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.focus",
        |host| {
            host.window.makeKeyAndOrderFront(None);
            Ok(())
        },
    )?;

    Ok(())
}

/// Raise one window.
pub(crate) unsafe fn window_raise(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    let runtime_state =
        validated_runtime_state(context, window_handle, "destack.display.window.raise")?;
    appkit_core::with_window_host(
        &runtime_state,
        window_handle,
        "destack.display.window.raise",
        |host| {
            host.window.orderFrontRegardless();
            Ok(())
        },
    )?;

    Ok(())
}

/// Start one interactive move-drag operation.
pub(crate) unsafe fn window_begin_move_drag(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    let _runtime_state = validated_runtime_state(
        context,
        window_handle,
        "destack.display.window.beginMoveDrag",
    )?;

    unsafe { unsupported::destack_display_window_begin_move_drag(context, window_handle) }
}

/// Start one interactive resize-drag operation.
pub(crate) unsafe fn window_begin_resize_drag(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    let _runtime_state = validated_runtime_state(
        context,
        window_handle,
        "destack.display.window.beginResizeDrag",
    )?;

    unsafe { unsupported::destack_display_window_begin_resize_drag(context, window_handle, edge) }
}
