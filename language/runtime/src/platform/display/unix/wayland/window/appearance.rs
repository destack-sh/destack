use std::io::Write;
use std::os::fd::AsFd;

use wayland_client::Proxy;
use wayland_client::protocol::wl_shm;
use wayland_protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_surface_v1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowChromeKind, WindowIconSet, WindowRole, WindowVisibility};
use crate::platform::{core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeStringRef};

use super::super::{core as backend_core, event};
use super::{
    create_memfd_file, decoration_mode_for_window, icon, normalize_opacity, opacity_multiplier,
    require_xdg_toplevel_id, resolve_window_binding,
};

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    always_on_top: bool,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding = resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // top-level and popup roles do not expose one portable always-on-top request lane
    if binding.role != WindowRole::Overlay {
        return Err(core_platform::not_supported(
            "destack.display.window.setAlwaysOnTop",
        ));
    }

    // overlay role is always topmost by definition: only true is accepted
    if !always_on_top {
        return Err(core_platform::not_supported(
            "destack.display.window.setAlwaysOnTop",
        ));
    }

    binding.always_on_top = true;

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding = resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setDecorated",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op decoration transitions
    if binding.decorated == decorated {
        return Ok(());
    }

    // require one negotiated decoration object for this window
    let Some(decoration_id) = binding.host.xdg_decoration.clone() else {
        return Err(core_platform::not_supported(
            "destack.display.window.setDecorated",
        ));
    };
    let chrome = binding.chrome;

    // apply one decoration mode request through xdg-decoration
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setDecorated",
        |connection, event_queue, _dispatch_state| {
            let decoration = zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1::from_id(
                connection,
                decoration_id,
            )
            .map_err(|error| {
                backend_core::io_error(
                    "destack.display.window.setDecorated",
                    format!("invalid zxdg_toplevel_decoration id: {error}"),
                )
            })?;

            let mode = decoration_mode_for_window(chrome, decorated);
            decoration.set_mode(mode);

            backend_core::flush_queue(event_queue, "destack.display.window.setDecorated")?;

            Ok(())
        },
    )?;

    binding.decorated = decorated;
    Ok(())
}

/// Set window resizable state.
pub(crate) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding = resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setResizable",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op resizable transitions
    if binding.resizable == resizable {
        return Ok(());
    }

    // apply wayland min and max size policy for this window
    let xdg_toplevel_id = require_xdg_toplevel_id(&binding, "destack.display.window.setResizable")?;
    let constraints = binding.constraints;
    let current_size = binding.size_physical;
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setResizable",
        |connection, event_queue, _dispatch_state| {
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.setResizable",
            )?;

            if !resizable {
                let width = current_size.width.min(i32::MAX as u32) as i32;
                let height = current_size.height.min(i32::MAX as u32) as i32;
                toplevel.set_min_size(width.max(1), height.max(1));
                toplevel.set_max_size(width.max(1), height.max(1));
            } else if let Some(constraints) = constraints {
                if let Some(minimum) = constraints.min {
                    let width = minimum.width.round().clamp(1.0, i32::MAX as f64) as i32;
                    let height = minimum.height.round().clamp(1.0, i32::MAX as f64) as i32;
                    toplevel.set_min_size(width, height);
                } else {
                    toplevel.set_min_size(0, 0);
                }

                if let Some(maximum) = constraints.max {
                    let width = maximum.width.round().clamp(1.0, i32::MAX as f64) as i32;
                    let height = maximum.height.round().clamp(1.0, i32::MAX as f64) as i32;
                    toplevel.set_max_size(width, height);
                } else {
                    toplevel.set_max_size(0, 0);
                }
            } else {
                toplevel.set_min_size(0, 0);
                toplevel.set_max_size(0, 0);
            }

            backend_core::flush_queue(event_queue, "destack.display.window.setResizable")?;

            Ok(())
        },
    )?;

    binding.resizable = resizable;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding =
        resolve_window_binding(context, window_handle, "destack.display.window.setChrome")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op chrome transitions
    if binding.chrome == chrome {
        return Ok(());
    }

    // require one negotiated decoration object for this window
    let Some(decoration_id) = binding.host.xdg_decoration.clone() else {
        return Err(core_platform::not_supported(
            "destack.display.window.setChrome",
        ));
    };

    let decorated = binding.decorated;
    let mode = decoration_mode_for_window(chrome, decorated);

    // apply one chrome-mode request through xdg-decoration
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setChrome",
        |connection, event_queue, _dispatch_state| {
            let decoration = zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1::from_id(
                connection,
                decoration_id,
            )
            .map_err(|error| {
                backend_core::io_error(
                    "destack.display.window.setChrome",
                    format!("invalid zxdg_toplevel_decoration id: {error}"),
                )
            })?;

            decoration.set_mode(mode);
            backend_core::flush_queue(event_queue, "destack.display.window.setChrome")?;

            Ok(())
        },
    )?;

    binding.chrome = chrome;

    Ok(())
}

/// Set one window taskbar visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding = resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setTaskbarVisible",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // top-level roles do not expose one portable taskbar-visibility request lane
    if binding.role == WindowRole::Toplevel {
        return Err(core_platform::not_supported(
            "destack.display.window.setTaskbarVisible",
        ));
    }

    // popup and overlay roles are transient surfaces and do not participate in task switching
    if visible {
        return Err(core_platform::not_supported(
            "destack.display.window.setTaskbarVisible",
        ));
    }

    binding.taskbar_visible = false;

    Ok(())
}

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    // decode title payload and resolve target window binding
    let title = unsafe { title.as_str()?.to_string() };
    let binding =
        resolve_window_binding(context, window_handle, "destack.display.window.setTitle")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // apply title request through xdg_toplevel
    let xdg_toplevel_id = require_xdg_toplevel_id(&binding, "destack.display.window.setTitle")?;
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setTitle",
        |connection, event_queue, _dispatch_state| {
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.setTitle",
            )?;

            toplevel.set_title(title.clone());
            backend_core::flush_queue(event_queue, "destack.display.window.setTitle")?;

            Ok(())
        },
    )?;

    binding.title = title;

    Ok(())
}

/// Set one window icon set.
pub(crate) unsafe fn window_set_icons(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    // validate and decode one icon payload before mutating window state
    icon::validate_icon_set(icons)?;
    let icon_buffer = icon::decode_icon_buffer(icons)?;

    // resolve target window binding and enforce owner-thread affinity
    let binding =
        resolve_window_binding(context, window_handle, "destack.display.window.setIcons")?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let xdg_toplevel_id = require_xdg_toplevel_id(&binding, "destack.display.window.setIcons")?;
    let surface_id = binding.host.surface.clone();
    let runtime_state = backend_core::runtime_state(context);
    let window_token = backend_core::window_token_from_surface(&runtime_state, &surface_id)
        .ok_or_else(|| {
            backend_core::io_error(
                "destack.display.window.setIcons",
                "missing wayland window dispatch token",
            )
        })?;

    // apply one icon update through xdg-toplevel-icon and wl_shm
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setIcons",
        |connection, event_queue, dispatch_state| {
            let manager = dispatch_state
                .toplevel_icon_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported("destack.display.window.setIcons"))?;
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.setIcons",
            )?;
            let surface = backend_core::resolve_wl_surface(
                connection,
                surface_id,
                "destack.display.window.setIcons",
            )?;

            // clear icon when no icon payload was provided
            if icon_buffer.is_none() {
                manager.set_icon(&toplevel, None);
                backend_core::request_surface_presentation_feedback(
                    dispatch_state,
                    event_queue,
                    &surface,
                    window_token.clone(),
                );
                surface.commit();
                backend_core::flush_queue(event_queue, "destack.display.window.setIcons")?;
                return Ok(());
            }

            // resolve one icon payload for shm upload
            let Some(icon_buffer) = icon_buffer.as_ref() else {
                return Err(core_platform::invalid_state(
                    "icon payload missing for wayland icon upload",
                ));
            };

            // upload icon pixels through one temporary wl_shm buffer
            let shm =
                dispatch_state.shm.as_ref().cloned().ok_or_else(|| {
                    core_platform::not_supported("destack.display.window.setIcons")
                })?;
            let byte_length = icon_buffer.pixels_argb8888.len();
            let byte_length_i32 = i32::try_from(byte_length).map_err(|_| {
                core_platform::invalid_argument("icons", "icon payload is too large")
            })?;

            let mut file = create_memfd_file(
                "destack.display.window.setIcons",
                "destack-wayland-icon",
                byte_length,
            )?;
            file.write_all(icon_buffer.pixels_argb8888.as_slice())
                .map_err(|error| {
                    backend_core::io_error(
                        "destack.display.window.setIcons",
                        format!("icon upload write failed: {error}"),
                    )
                })?;
            file.flush().map_err(|error| {
                backend_core::io_error(
                    "destack.display.window.setIcons",
                    format!("icon upload flush failed: {error}"),
                )
            })?;

            let queue_handle = event_queue.handle();
            let pool = shm.create_pool(file.as_fd(), byte_length_i32, &queue_handle, ());
            let buffer = pool.create_buffer(
                0,
                icon_buffer.width,
                icon_buffer.height,
                icon_buffer.stride,
                wl_shm::Format::Argb8888,
                &queue_handle,
                (),
            );
            let icon = manager.create_icon(&queue_handle, ());
            icon.add_buffer(&buffer, 1);
            manager.set_icon(&toplevel, Some(&icon));
            backend_core::request_surface_presentation_feedback(
                dispatch_state,
                event_queue,
                &surface,
                window_token.clone(),
            );
            surface.commit();
            icon.destroy();
            buffer.destroy();
            pool.destroy();

            backend_core::flush_queue(event_queue, "destack.display.window.setIcons")?;

            Ok(())
        },
    )
}

/// Set one window visibility state.
pub(crate) unsafe fn window_set_visibility(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding = resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setVisibility",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op visibility transitions
    let previous_visibility = binding.visibility;
    if previous_visibility == visibility {
        return Ok(());
    }

    // reject hidden visibility transitions: no portable wayland request lane exists
    if visibility == WindowVisibility::Hidden {
        return Err(core_platform::not_supported(
            "destack.display.window.setVisibility",
        ));
    }

    // apply visibility request to xdg_toplevel
    let xdg_toplevel_id =
        require_xdg_toplevel_id(&binding, "destack.display.window.setVisibility")?;
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setVisibility",
        |connection, event_queue, _dispatch_state| {
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.setVisibility",
            )?;

            match visibility {
                WindowVisibility::Minimized => {
                    toplevel.set_minimized();
                }
                WindowVisibility::Maximized => {
                    toplevel.set_maximized();
                }
                WindowVisibility::Visible => {
                    toplevel.unset_fullscreen();
                    toplevel.unset_maximized();
                }
                WindowVisibility::Hidden => {
                    return Err(core_platform::not_supported(
                        "destack.display.window.setVisibility",
                    ));
                }
            }

            backend_core::flush_queue(event_queue, "destack.display.window.setVisibility")?;

            Ok(())
        },
    )?;

    binding.visibility = visibility;
    drop(binding);

    let runtime_state = backend_core::runtime_state(context);
    event::publish_window_visibility_changed(
        &runtime_state,
        window_handle,
        previous_visibility,
        visibility,
    );

    Ok(())
}

/// Set one whole-window opacity value.
pub(crate) unsafe fn window_set_opacity(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    // normalize opacity payload and resolve target window binding
    let opacity = normalize_opacity(opacity, "opacity")?;
    let binding =
        resolve_window_binding(context, window_handle, "destack.display.window.setOpacity")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op opacity transitions
    if (binding.opacity - opacity).abs() <= f64::EPSILON {
        return Ok(());
    }

    // apply opacity through one optional alpha-modifier surface lane
    let surface_id = binding.host.surface.clone();
    let alpha_modifier_surface_id = binding.host.alpha_modifier_surface.clone();
    let runtime_state = backend_core::runtime_state(context);
    let window_token = backend_core::window_token_from_surface(&runtime_state, &surface_id)
        .ok_or_else(|| {
            backend_core::io_error(
                "destack.display.window.setOpacity",
                "missing wayland window dispatch token",
            )
        })?;
    let next_alpha_modifier_surface_id = backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setOpacity",
        |connection, event_queue, dispatch_state| {
            // require one negotiated alpha-modifier manager
            let manager = dispatch_state
                .alpha_modifier_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported("destack.display.window.setOpacity"))?;

            // resolve this window surface for the opacity update
            let surface = backend_core::resolve_wl_surface(
                connection,
                surface_id.clone(),
                "destack.display.window.setOpacity",
            )?;
            let mut alpha_modifier_surface_id = alpha_modifier_surface_id.clone();

            // create one alpha-modifier surface object on first use
            if alpha_modifier_surface_id.is_none() {
                let alpha_surface = manager.get_surface(&surface, &event_queue.handle(), ());
                alpha_modifier_surface_id = Some(alpha_surface.id());
            }

            // resolve alpha-surface object and set the requested multiplier
            let alpha_surface_id = alpha_modifier_surface_id
                .ok_or_else(|| core_platform::not_supported("destack.display.window.setOpacity"))?;
            let alpha_surface = wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1::from_id(
                connection,
                alpha_surface_id.clone(),
            )
            .map_err(|error| {
                backend_core::io_error(
                    "destack.display.window.setOpacity",
                    format!("invalid wp_alpha_modifier_surface id: {error}"),
                )
            })?;
            alpha_surface.set_multiplier(opacity_multiplier(opacity));
            backend_core::request_surface_presentation_feedback(
                dispatch_state,
                event_queue,
                &surface,
                window_token.clone(),
            );
            surface.commit();
            backend_core::flush_queue(event_queue, "destack.display.window.setOpacity")?;

            Ok(Some(alpha_surface_id))
        },
    )?;

    // update runtime opacity snapshots after protocol commit succeeds
    binding.opacity = opacity;
    binding.host.alpha_modifier_surface = next_alpha_modifier_surface_id;

    Ok(())
}

/// Read one whole-window opacity value.
pub(crate) unsafe fn window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve target window binding
    core_platform::ensure_out(out, "out")?;
    let binding = resolve_window_binding(context, window_handle, "destack.display.window.opacity")?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // write current opacity snapshot
    unsafe {
        *out = binding.opacity;
    }

    Ok(())
}
