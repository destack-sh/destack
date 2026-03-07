use std::sync::{Arc, Mutex};

use objc2_app_kit::NSWindow;

use super::core::{display_id_from_screen, occlusion_from_window, refresh_host_state_geometry};
use super::cursor::apply_cursor_policy;
use crate::diagnostic::RuntimeResult;
use crate::platform::display::unix::appkit::event::publish_state_deltas;
use crate::platform::display::unix::appkit::model::AppKitWindowHostState;
use crate::platform::display::unix::appkit::{core as appkit_core, resource as display_resource};
use crate::platform::display::{WindowTheme, WindowVisibility};
use crate::platform::resource;

/// Reconcile one requested visibility override against the current host snapshot.
fn reconcile_requested_visibility(host_state: &mut AppKitWindowHostState) {
    let host_visibility = host_state.visibility;

    // keep the last non-minimized target so restore can roundtrip correctly
    if host_visibility != WindowVisibility::Minimized {
        host_state.restored_visibility = host_visibility;
    }

    let Some(requested_visibility) = host_state.requested_visibility else {
        return;
    };

    // clear the request once the host catches up to the desired state
    if host_visibility == requested_visibility {
        if requested_visibility != WindowVisibility::Minimized {
            host_state.restored_visibility = requested_visibility;
        }

        host_state.requested_visibility = None;
        return;
    }

    // otherwise keep the requested state visible to callers until AppKit updates
    host_state.visibility = requested_visibility;
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

/// Refresh one host-state snapshot from one native host window.
fn refresh_host_window_snapshot(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    host_state: &mut AppKitWindowHostState,
    window: &NSWindow,
) {
    refresh_host_state_geometry(host_state, window);
    host_state.display = display_handle_from_window(runtime_state, window);
    reconcile_requested_visibility(host_state);
}

/// Apply one host position change emitted by the native window system.
pub(crate) fn apply_host_position_change(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    host_state: &Arc<Mutex<AppKitWindowHostState>>,
    window: &NSWindow,
) {
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    refresh_host_window_snapshot(runtime_state, &mut host_state, window);
    let next = host_state.clone();
    drop(host_state);

    publish_state_deltas(runtime_state, window_handle, &previous, &next);
}

/// Apply one host size change emitted by the native window system.
pub(crate) fn apply_host_size_change(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    host_state: &Arc<Mutex<AppKitWindowHostState>>,
    window: &NSWindow,
) {
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    refresh_host_window_snapshot(runtime_state, &mut host_state, window);
    let next = host_state.clone();
    drop(host_state);

    publish_state_deltas(runtime_state, window_handle, &previous, &next);
}

/// Apply one host focus change emitted by the native window system.
pub(crate) fn apply_host_focus_change(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    host_state: &Arc<Mutex<AppKitWindowHostState>>,
    focused: bool,
) {
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    host_state.focused = focused;
    let next = host_state.clone();
    drop(host_state);

    publish_state_deltas(runtime_state, window_handle, &previous, &next);

    apply_cursor_policy(runtime_state);
}

/// Apply one host occlusion change emitted by the native window system.
pub(crate) fn apply_host_occlusion_change(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
    host_state: &Arc<Mutex<AppKitWindowHostState>>,
    window: &NSWindow,
) {
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    host_state.occlusion = occlusion_from_window(window);
    let next = host_state.clone();
    drop(host_state);

    publish_state_deltas(runtime_state, window_handle, &previous, &next);
}

/// Refresh one host-state snapshot from the native host window.
pub(crate) fn refresh_host_window_binding(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<(AppKitWindowHostState, AppKitWindowHostState)> {
    appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.refreshHostState",
        |host| {
            let mut host_state = host
                .host_state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let previous = host_state.clone();

            refresh_host_window_snapshot(runtime_state, &mut host_state, &host.window);

            let next = host_state.clone();
            drop(host_state);

            Ok((previous, next))
        },
    )
}

/// Refresh the global theme snapshot for all opened host windows.
pub(crate) fn reconcile_all_host_window_themes(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    theme: WindowTheme,
) {
    let window_entries = appkit_core::with_main_thread_state(runtime_state, |state| {
        state
            .borrow()
            .windows
            .iter()
            .map(|(window_handle, host)| (*window_handle, Arc::clone(&host.host_state)))
            .collect::<Vec<(resource::WindowHandle, Arc<Mutex<AppKitWindowHostState>>)>>()
    });

    // update only the global theme field for each opened window
    for (window_handle, host_state) in window_entries {
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        let previous = host_state.clone();

        // skip windows that already reflect the current application theme
        if host_state.theme == theme {
            continue;
        }

        host_state.theme = theme;
        let next = host_state.clone();
        drop(host_state);

        publish_state_deltas(runtime_state, window_handle, &previous, &next);
    }
}

/// Refresh one host-state snapshot from the native host window and publish state deltas.
pub(crate) fn reconcile_host_window_state(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    reconcile_host_window_state_with_visibility(runtime_state, window_handle, None)
}

/// Refresh one host-state snapshot from the native host window and apply one optional visibility fallback.
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
                let mut host_state = host
                    .host_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());

                if visibility == WindowVisibility::Minimized {
                    if host_state.visibility != WindowVisibility::Minimized {
                        host_state.restored_visibility = host_state.visibility;
                    }
                } else {
                    host_state.restored_visibility = visibility;
                }

                host_state.requested_visibility = Some(visibility);

                Ok(())
            },
        )?;
    }

    let (previous, next) = refresh_host_window_binding(runtime_state, window_handle)?;

    publish_state_deltas(runtime_state, window_handle, &previous, &next);

    apply_cursor_policy(runtime_state);

    Ok(())
}
