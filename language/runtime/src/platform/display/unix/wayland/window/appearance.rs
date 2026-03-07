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

use super::{
    create_memfd_file, decoration_mode_for_window, icon, normalize_opacity, opacity_multiplier,
    require_xdg_toplevel_id, resolve_window_host_state,
};
use crate::platform::display::unix::wayland::{core as wayland_core, event};

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    always_on_top: bool,
) -> RuntimeResult<()> {
    // resolve target window host state and mutate host state
    let host_state = resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // top-level and popup roles do not expose one portable always-on-top request lane
    if host_state.role != WindowRole::Overlay {
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

    host_state.always_on_top = true;

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    // resolve target window host state and mutate host state
    let host_state = resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setDecorated",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op decoration transitions
    if host_state.decorated == decorated {
        return Ok(());
    }

    // require one negotiated decoration object for this window
    let Some(decoration_id) = host_state.host.xdg_decoration.clone() else {
        return Err(core_platform::not_supported(
            "destack.display.window.setDecorated",
        ));
    };
    let chrome = host_state.chrome;

    // apply one decoration mode request through xdg-decoration
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.setDecorated",
        |connection, event_queue, _dispatch_state| {
            let decoration = zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1::from_id(
                connection,
                decoration_id,
            )
            .map_err(|error| {
                wayland_core::io_error(
                    "destack.display.window.setDecorated",
                    format!("invalid zxdg_toplevel_decoration id: {error}"),
                )
            })?;

            let mode = decoration_mode_for_window(chrome, decorated);
            decoration.set_mode(mode);

            wayland_core::flush_queue(event_queue, "destack.display.window.setDecorated")?;

            Ok(())
        },
    )?;

    host_state.decorated = decorated;
    Ok(())
}

/// Set window resizable state.
pub(crate) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    // resolve target window host state and mutate host state
    let host_state = resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setResizable",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op resizable transitions
    if host_state.resizable == resizable {
        return Ok(());
    }

    // apply wayland min and max size policy for this window
    let xdg_toplevel_id =
        require_xdg_toplevel_id(&host_state, "destack.display.window.setResizable")?;
    let constraints = host_state.constraints;
    let current_size = host_state.size_physical;
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.setResizable",
        |connection, event_queue, _dispatch_state| {
            let toplevel = wayland_core::resolve_xdg_toplevel(
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

            wayland_core::flush_queue(event_queue, "destack.display.window.setResizable")?;

            Ok(())
        },
    )?;

    host_state.resizable = resizable;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    // resolve target window host state and mutate host state
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.setChrome")?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // skip no-op chrome transitions
    if host_state.chrome == chrome {
        return Ok(());
    }

    // require one negotiated decoration object for this window
    let Some(decoration_id) = host_state.host.xdg_decoration.clone() else {
        return Err(core_platform::not_supported(
            "destack.display.window.setChrome",
        ));
    };

    let decorated = host_state.decorated;
    let mode = decoration_mode_for_window(chrome, decorated);

    // apply one chrome-mode request through xdg-decoration
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.setChrome",
        |connection, event_queue, _dispatch_state| {
            let decoration = zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1::from_id(
                connection,
                decoration_id,
            )
            .map_err(|error| {
                wayland_core::io_error(
                    "destack.display.window.setChrome",
                    format!("invalid zxdg_toplevel_decoration id: {error}"),
                )
            })?;

            decoration.set_mode(mode);
            wayland_core::flush_queue(event_queue, "destack.display.window.setChrome")?;

            Ok(())
        },
    )?;

    host_state.chrome = chrome;
    let current = host_state.clone();
    drop(host_state);

    // publish all affected state deltas
    let runtime_state = wayland_core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Set one window taskbar visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve target window host state and mutate host state
    let host_state = resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setTaskbarVisible",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // top-level roles do not expose one portable taskbar-visibility request lane
    if host_state.role == WindowRole::Toplevel {
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

    host_state.taskbar_visible = false;
    let current = host_state.clone();
    drop(host_state);

    // publish all affected state deltas
    let runtime_state = wayland_core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    // decode title payload and resolve target window host state
    let title = unsafe { title.as_str()?.to_string() };
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.setTitle")?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply title request through xdg_toplevel
    let xdg_toplevel_id = require_xdg_toplevel_id(&host_state, "destack.display.window.setTitle")?;
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.setTitle",
        |connection, event_queue, _dispatch_state| {
            let toplevel = wayland_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.setTitle",
            )?;

            toplevel.set_title(title.clone());
            wayland_core::flush_queue(event_queue, "destack.display.window.setTitle")?;

            Ok(())
        },
    )?;

    host_state.title = title;

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

    // resolve target window host state and mutate host state
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.setIcons")?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let xdg_toplevel_id = require_xdg_toplevel_id(&host_state, "destack.display.window.setIcons")?;
    let surface_id = host_state.host.surface.clone();
    let runtime_state = wayland_core::runtime_state(context);
    let window_token = runtime_state
        .window_token_from_surface(&surface_id)
        .ok_or_else(|| {
            wayland_core::io_error(
                "destack.display.window.setIcons",
                "missing wayland window dispatch token",
            )
        })?;

    // apply one icon update through xdg-toplevel-icon and wl_shm
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.setIcons",
        |connection, event_queue, dispatch_state| {
            let manager = dispatch_state
                .globals
                .toplevel_icon_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported("destack.display.window.setIcons"))?;
            let toplevel = wayland_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.setIcons",
            )?;
            let surface = wayland_core::resolve_wl_surface(
                connection,
                surface_id,
                "destack.display.window.setIcons",
            )?;

            // clear icon when no icon payload was provided
            if icon_buffer.is_none() {
                manager.set_icon(&toplevel, None);
                wayland_core::request_surface_presentation_feedback(
                    dispatch_state,
                    event_queue,
                    &surface,
                    window_token.clone(),
                );
                surface.commit();
                wayland_core::flush_queue(event_queue, "destack.display.window.setIcons")?;
                return Ok(());
            }

            // resolve one icon payload for shm upload
            let Some(icon_buffer) = icon_buffer.as_ref() else {
                return Err(core_platform::invalid_state(
                    "icon payload missing for wayland icon upload",
                ));
            };

            // upload icon pixels through one temporary wl_shm buffer
            let shm = dispatch_state
                .globals
                .shm
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported("destack.display.window.setIcons"))?;
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
                    wayland_core::io_error(
                        "destack.display.window.setIcons",
                        format!("icon upload write failed: {error}"),
                    )
                })?;
            file.flush().map_err(|error| {
                wayland_core::io_error(
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
            wayland_core::request_surface_presentation_feedback(
                dispatch_state,
                event_queue,
                &surface,
                window_token.clone(),
            );
            surface.commit();
            icon.destroy();
            buffer.destroy();
            pool.destroy();

            wayland_core::flush_queue(event_queue, "destack.display.window.setIcons")?;

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
    // resolve target window host state and mutate host state
    let host_state = resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setVisibility",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op visibility transitions
    let previous_visibility = host_state.visibility;
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
        require_xdg_toplevel_id(&host_state, "destack.display.window.setVisibility")?;
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.setVisibility",
        |connection, event_queue, _dispatch_state| {
            let toplevel = wayland_core::resolve_xdg_toplevel(
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

            wayland_core::flush_queue(event_queue, "destack.display.window.setVisibility")?;

            Ok(())
        },
    )?;

    host_state.visibility = visibility;
    drop(host_state);

    let runtime_state = wayland_core::runtime_state(context);
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
    // normalize opacity payload and resolve target window host state
    let opacity = normalize_opacity(opacity, "opacity")?;
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.setOpacity")?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // skip no-op opacity transitions
    if (host_state.opacity - opacity).abs() <= f64::EPSILON {
        return Ok(());
    }

    // apply opacity through one optional alpha-modifier surface lane
    let surface_id = host_state.host.surface.clone();
    let alpha_modifier_surface_id = host_state.host.alpha_modifier_surface.clone();
    let runtime_state = wayland_core::runtime_state(context);
    let window_token = runtime_state
        .window_token_from_surface(&surface_id)
        .ok_or_else(|| {
            wayland_core::io_error(
                "destack.display.window.setOpacity",
                "missing wayland window dispatch token",
            )
        })?;
    let next_alpha_modifier_surface_id = wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.setOpacity",
        |connection, event_queue, dispatch_state| {
            // require one negotiated alpha-modifier manager
            let manager = dispatch_state
                .globals
                .alpha_modifier_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported("destack.display.window.setOpacity"))?;

            // resolve the target surface for the opacity update
            let surface = wayland_core::resolve_wl_surface(
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
                wayland_core::io_error(
                    "destack.display.window.setOpacity",
                    format!("invalid wp_alpha_modifier_surface id: {error}"),
                )
            })?;
            alpha_surface.set_multiplier(opacity_multiplier(opacity));
            wayland_core::request_surface_presentation_feedback(
                dispatch_state,
                event_queue,
                &surface,
                window_token.clone(),
            );
            surface.commit();
            wayland_core::flush_queue(event_queue, "destack.display.window.setOpacity")?;

            Ok(Some(alpha_surface_id))
        },
    )?;

    // update runtime opacity snapshots after protocol commit succeeds
    host_state.opacity = opacity;
    host_state.host.alpha_modifier_surface = next_alpha_modifier_surface_id;
    let current = host_state.clone();
    drop(host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Read one whole-window opacity value.
pub(crate) unsafe fn window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve target window host state
    core_platform::ensure_out(out, "out")?;
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.opacity")?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // write current opacity snapshot
    unsafe {
        *out = host_state.opacity;
    }

    Ok(())
}
