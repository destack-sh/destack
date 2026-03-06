use std::sync::{Arc, Mutex};

use objc2_app_kit::NSWindow;

use super::super::event::publish_state_deltas;
use super::super::model::AppKitWindowBinding;
use super::super::{core as appkit_core, resource as display_resource};
use super::core::{display_id_from_screen, occlusion_from_window, refresh_binding_geometry};
use super::cursor::apply_cursor_policy;
use crate::diagnostic::RuntimeResult;
use crate::platform::display::WindowVisibility;
use crate::platform::resource;

/// Reconcile one requested visibility override against the current host snapshot.
fn reconcile_requested_visibility(binding: &mut AppKitWindowBinding) {
    let host_visibility = binding.visibility;

    // keep the last non-minimized target so restore can roundtrip correctly
    if host_visibility != WindowVisibility::Minimized {
        binding.restored_visibility = host_visibility;
    }

    let Some(requested_visibility) = binding.requested_visibility else {
        return;
    };

    // clear the request once the host catches up to the desired state
    if host_visibility == requested_visibility {
        if requested_visibility != WindowVisibility::Minimized {
            binding.restored_visibility = requested_visibility;
        }

        binding.requested_visibility = None;
        return;
    }

    // otherwise keep the requested state visible to callers until AppKit updates
    binding.visibility = requested_visibility;
}

/// Resolve one display handle from the current native host screen.
fn display_handle_from_window(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: &NSWindow,
) -> Option<resource::DisplayHandle> {
    let display_id = window
        .screen()
        .and_then(|screen| display_id_from_screen(&screen))?;

    Some(display_resource::cached_display_handle_for_runtime(
        runtime_state,
        &display_id,
    ))
}

/// Refresh one binding snapshot from one native host window.
fn refresh_host_window_snapshot(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    binding: &mut AppKitWindowBinding,
    window: &NSWindow,
) {
    refresh_binding_geometry(binding, window);
    binding.display = display_handle_from_window(runtime_state, window);
    reconcile_requested_visibility(binding);
}

/// Apply one host position change emitted by the native window system.
pub(crate) fn apply_host_position_change(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    binding: &Arc<Mutex<AppKitWindowBinding>>,
    window: &NSWindow,
) {
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let previous = binding.clone();
    refresh_host_window_snapshot(runtime_state, &mut binding, window);
    let next = binding.clone();
    drop(binding);

    publish_state_deltas(runtime_state, window_handle, &previous, &next);
}

/// Apply one host size change emitted by the native window system.
pub(crate) fn apply_host_size_change(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    binding: &Arc<Mutex<AppKitWindowBinding>>,
    window: &NSWindow,
) {
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let previous = binding.clone();
    refresh_host_window_snapshot(runtime_state, &mut binding, window);
    let next = binding.clone();
    drop(binding);

    publish_state_deltas(runtime_state, window_handle, &previous, &next);
}

/// Apply one host focus change emitted by the native window system.
pub(crate) fn apply_host_focus_change(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    binding: &Arc<Mutex<AppKitWindowBinding>>,
    focused: bool,
) {
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let previous = binding.clone();
    binding.focused = focused;
    let next = binding.clone();
    drop(binding);

    publish_state_deltas(runtime_state, window_handle, &previous, &next);

    apply_cursor_policy(runtime_state);
}

/// Apply one host occlusion change emitted by the native window system.
pub(crate) fn apply_host_occlusion_change(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    binding: &Arc<Mutex<AppKitWindowBinding>>,
    window: &NSWindow,
) {
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let previous = binding.clone();
    binding.occlusion = occlusion_from_window(window);
    let next = binding.clone();
    drop(binding);

    publish_state_deltas(runtime_state, window_handle, &previous, &next);
}

/// Refresh one binding snapshot from the native host window.
pub(crate) fn refresh_host_window_binding(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<(AppKitWindowBinding, AppKitWindowBinding)> {
    appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.refreshHostState",
        |host| {
            let mut binding = host
                .binding
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let previous = binding.clone();

            refresh_host_window_snapshot(runtime_state, &mut binding, &host.window);

            let next = binding.clone();
            drop(binding);

            Ok((previous, next))
        },
    )
}

/// Refresh all opened host windows and publish any state deltas.
pub(crate) fn reconcile_all_host_window_bindings(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
) -> RuntimeResult<()> {
    let window_handles = appkit_core::with_main_thread_state(runtime_state, |state| {
        state
            .borrow()
            .windows
            .keys()
            .copied()
            .collect::<Vec<resource::WindowHandle>>()
    });

    // refresh every opened window so host-driven state stays authoritative
    for window_handle in window_handles {
        let (previous, next) = refresh_host_window_binding(runtime_state, window_handle)?;
        publish_state_deltas(runtime_state, window_handle, &previous, &next);
    }

    apply_cursor_policy(runtime_state);

    Ok(())
}

/// Refresh one binding snapshot from the native host window and publish state deltas.
pub(crate) fn reconcile_host_window_state(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    reconcile_host_window_state_with_visibility(runtime_state, window_handle, None)
}

/// Refresh one binding snapshot from the native host window and apply one optional visibility fallback.
pub(crate) fn reconcile_host_window_state_with_visibility(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    visibility: Option<WindowVisibility>,
) -> RuntimeResult<()> {
    // persist the requested state so reads remain stable while AppKit updates asynchronously
    if let Some(visibility) = visibility {
        appkit_core::with_window_host(
            runtime_state,
            window_handle,
            "destack.display.window.reconcileHostState",
            |host| {
                let mut binding = host
                    .binding
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());

                if visibility == WindowVisibility::Minimized {
                    if binding.visibility != WindowVisibility::Minimized {
                        binding.restored_visibility = binding.visibility;
                    }
                } else {
                    binding.restored_visibility = visibility;
                }

                binding.requested_visibility = Some(visibility);

                Ok(())
            },
        )?;
    }

    let (previous, next) = refresh_host_window_binding(runtime_state, window_handle)?;

    publish_state_deltas(runtime_state, window_handle, &previous, &next);

    apply_cursor_policy(runtime_state);

    Ok(())
}
