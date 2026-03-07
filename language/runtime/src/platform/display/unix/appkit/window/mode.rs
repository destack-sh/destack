use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::WindowModeOptions;
use crate::platform::resource::{DisplayHandle, WindowHandle};
use crate::runtime::BindingCallContext;

use super::reconcile;
use crate::platform::display::unix::appkit::event::publish_state_deltas;
use crate::platform::display::unix::appkit::{
    core as appkit_core, monitor, resource as display_resource,
};

/// Return whether one AppKit window is currently in native fullscreen mode.
fn is_fullscreen_window(window: &NSWindow) -> bool {
    window.styleMask().contains(NSWindowStyleMask::FullScreen)
}

/// Return one target display from one window mode payload.
pub(crate) fn mode_display(mode: WindowModeOptions) -> Option<DisplayHandle> {
    match mode {
        WindowModeOptions::WindowWindowedModeOptions(_) => None,
        WindowModeOptions::WindowBorderlessModeOptions(options) => options.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(options) => Some(options.display),
    }
}

/// Resolve one display frame for one runtime display handle.
pub(crate) fn display_frame(
    context: &BindingCallContext,
    display: DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<NSRect> {
    let display_id = display_resource::resolve_display_id(context, display, operation)?;
    let snapshot = monitor::monitor_snapshot_by_display_id(&display_id)?.ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display id `{display_id}` is no longer available"),
        )
    })?;

    Ok(NSRect::new(
        NSPoint::new(snapshot.descriptor.x as f64, snapshot.descriptor.y as f64),
        NSSize::new(
            snapshot.descriptor.width_px.max(1) as f64,
            snapshot.descriptor.height_px.max(1) as f64,
        ),
    ))
}

/// Resolve one target frame for one window mode transition.
fn target_frame_for_mode(
    context: &BindingCallContext,
    mode: WindowModeOptions,
    fallback_display: Option<DisplayHandle>,
    operation: &'static str,
) -> RuntimeResult<Option<NSRect>> {
    let display = mode_display(mode).or(fallback_display);
    let Some(display) = display else {
        return Ok(None);
    };

    Ok(Some(display_frame(context, display, operation)?))
}

/// Compare two mode payloads by semantic fields.
pub(crate) fn same_window_mode(left: WindowModeOptions, right: WindowModeOptions) -> bool {
    match (left, right) {
        (
            WindowModeOptions::WindowWindowedModeOptions(_),
            WindowModeOptions::WindowWindowedModeOptions(_),
        ) => true,
        (
            WindowModeOptions::WindowBorderlessModeOptions(left),
            WindowModeOptions::WindowBorderlessModeOptions(right),
        ) => left.display == right.display,
        (
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(left),
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(right),
        ) => left.display == right.display && left.display_mode == right.display_mode,
        _ => false,
    }
}

/// Set one window mode.
pub(crate) unsafe fn window_set_mode(
    binding: &BindingCallContext,
    window: WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    let runtime_state = appkit_core::runtime_state(binding);
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.setMode",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let previous = resolved_host_state.clone();

    // accept exact no-op transitions without mutating host state
    if same_window_mode(previous.mode, mode) {
        return Ok(());
    }

    let target_frame = target_frame_for_mode(
        binding,
        mode,
        resolved_host_state.display,
        "destack.display.window.setMode",
    )?;

    // validate any explicit display target before rejecting unsupported exclusive fullscreen
    if matches!(
        mode,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
    ) {
        return Err(core_platform::not_supported(
            "destack.display.window.setMode",
        ));
    }

    appkit_core::with_window_host(
        &runtime_state,
        window,
        "destack.display.window.setMode",
        |host| {
            if let Some(frame) = target_frame {
                host.window.setFrame_display(frame, true);
            }

            let mut collection_behavior = host.window.collectionBehavior();
            collection_behavior.insert(NSWindowCollectionBehavior::FullScreenPrimary);
            host.window.setCollectionBehavior(collection_behavior);

            match mode {
                WindowModeOptions::WindowWindowedModeOptions(_) => {
                    if is_fullscreen_window(&host.window) {
                        host.window.toggleFullScreen(None);
                    }
                }
                WindowModeOptions::WindowBorderlessModeOptions(_) => {
                    if !is_fullscreen_window(&host.window) {
                        host.window.toggleFullScreen(None);
                    }
                }
                WindowModeOptions::WindowExclusiveFullscreenModeOptions(_) => unreachable!(),
            }

            Ok(())
        },
    )?;

    resolved_host_state.mode = mode;
    resolved_host_state.display = mode_display(mode).or(resolved_host_state.display);
    drop(resolved_host_state);

    let (_, next) = reconcile::refresh_host_window_binding(&runtime_state, window)?;

    publish_state_deltas(&runtime_state, window, &previous, &next);
    Ok(())
}
