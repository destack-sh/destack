use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as XprotoConnectionExt, PropMode};
use x11rb::wrapper::ConnectionExt as X11WrapperConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{WindowChromeKind, WindowIconSet, WindowVisibility};
use crate::platform::resource::WindowHandle;
use crate::runtime::{BindingCallContext, NativeStringRef};

use super::icon::net_wm_icon_payload;
use super::{
    apply_window_chrome, apply_window_decorated, apply_window_mouse_passthrough,
    apply_window_size_hints, request_window_minimize, set_net_wm_state, set_window_title,
};
use crate::platform::display::unix::x11::{core, event, resource as display_resource};

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    always_on_top: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate the host state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setAlwaysOnTop")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    set_net_wm_state(
        connection_state.as_ref(),
        host_state.window,
        connection_state.atoms.net_wm_state_above,
        always_on_top,
    )?;
    drop(host_state);
    runtime_state.service_ingress("destack.display.window.setAlwaysOnTop")?;

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate the host state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setDecorated")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setDecorated",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply one host decoration mutation before updating the snapshot
    apply_window_decorated(
        connection_state.as_ref(),
        host_state.window,
        decorated,
        "destack.display.window.setDecorated",
    )?;
    host_state.decorated = decorated;

    Ok(())
}

/// Set window resizable state.
pub(crate) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate the host state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setResizable")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setResizable",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply one host size-hints mutation before updating the snapshot
    apply_window_size_hints(
        connection_state.as_ref(),
        host_state.window,
        resizable,
        host_state.constraints,
        host_state.aspect_ratio,
        host_state.size_physical,
        "destack.display.window.setResizable",
    )?;
    host_state.resizable = resizable;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    // resolve runtime and mutate the host state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setChrome")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setChrome",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // apply one host window-type mutation before updating the snapshot
    apply_window_chrome(
        connection_state.as_ref(),
        host_state.window,
        chrome,
        "destack.display.window.setChrome",
    )?;
    host_state.chrome = chrome;
    let current = host_state.clone();
    drop(host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Set one window icon set.
pub(crate) unsafe fn window_set_icons(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    // resolve runtime and window host state lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setIcons")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setIcons",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply icon property update or clear it when no icons are configured
    if let Some(icons) = icons {
        let payload = net_wm_icon_payload(icons)?;
        connection_state
            .connection
            .change_property32(
                PropMode::REPLACE,
                host_state.window,
                connection_state.atoms.net_wm_icon,
                AtomEnum::CARDINAL,
                &payload,
            )
            .map_err(|error| {
                core::io_error(
                    "destack.display.window.setIcons",
                    format!("change_property32 failed: {error}"),
                )
            })?;
    } else {
        connection_state
            .connection
            .delete_property(host_state.window, connection_state.atoms.net_wm_icon)
            .map_err(|error| {
                core::io_error(
                    "destack.display.window.setIcons",
                    format!("delete_property failed: {error}"),
                )
            })?;
    }
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setIcons",
            format!("flush failed: {error}"),
        )
    })?;

    Ok(())
}

/// Set one window mouse-passthrough state.
pub(crate) unsafe fn window_set_mouse_passthrough(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate the host state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setMousePassthrough")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // apply one host input-shape mutation before updating the snapshot
    apply_window_mouse_passthrough(
        connection_state.as_ref(),
        host_state.window,
        passthrough,
        "destack.display.window.setMousePassthrough",
    )?;
    host_state.mouse_passthrough = passthrough;
    let current = host_state.clone();
    drop(host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Set one window opacity.
pub(crate) unsafe fn window_set_opacity(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    // validate opacity range
    if !(0.0..=1.0).contains(&opacity) {
        return Err(core_platform::invalid_argument(
            "opacity",
            "window opacity must be between 0.0 and 1.0",
        ));
    }

    // resolve runtime and mutate the host state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setOpacity")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setOpacity",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    host_state.opacity = opacity;

    // apply EWMH opacity property on the window
    let encoded = (opacity.clamp(0.0, 1.0) * (u32::MAX as f64)).round() as u32;
    connection_state
        .connection
        .change_property32(
            PropMode::REPLACE,
            host_state.window,
            connection_state.atoms.net_wm_window_opacity,
            AtomEnum::CARDINAL,
            &[encoded],
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setOpacity",
                format!("change_property32 failed: {error}"),
            )
        })?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setOpacity",
            format!("flush failed: {error}"),
        )
    })?;
    let current = host_state.clone();
    drop(host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Read one window opacity.
pub(crate) unsafe fn window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // validate the out pointer and read the host state
    core_platform::ensure_out(out, "out")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.opacity",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        *out = host_state.opacity;
    }

    Ok(())
}

/// Set one window taskbar visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate the host state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setTaskbarVisible")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setTaskbarVisible",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    set_net_wm_state(
        connection_state.as_ref(),
        host_state.window,
        connection_state.atoms.net_wm_state_skip_taskbar,
        !visible,
    )?;
    drop(host_state);
    runtime_state.service_ingress("destack.display.window.setTaskbarVisible")?;

    Ok(())
}

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    // parse the title payload and resolve the host state
    let title = unsafe { title.as_str()? };
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setTitle")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setTitle",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply host title updates and mutate host_state snapshot
    set_window_title(connection_state.as_ref(), host_state.window, title)?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setTitle",
            format!("flush failed: {error}"),
        )
    })?;
    host_state.title = title.to_string();

    Ok(())
}

/// Set one window visibility state.
pub(crate) unsafe fn window_set_visibility(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    // resolve runtime and host_state lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setVisibility")?;
    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setVisibility",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply host visibility transitions
    match visibility {
        WindowVisibility::Hidden => {
            connection_state
                .connection
                .unmap_window(host_state.window)
                .map_err(|error| {
                    core::io_error(
                        "destack.display.window.setVisibility",
                        format!("unmap_window failed: {error}"),
                    )
                })?;
        }
        WindowVisibility::Visible | WindowVisibility::Maximized => {
            connection_state
                .connection
                .map_window(host_state.window)
                .map_err(|error| {
                    core::io_error(
                        "destack.display.window.setVisibility",
                        format!("map_window failed: {error}"),
                    )
                })?;
        }
        WindowVisibility::Minimized => {
            request_window_minimize(
                connection_state.as_ref(),
                host_state.window,
                "destack.display.window.setVisibility",
            )?;
        }
    }
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setVisibility",
            format!("flush failed: {error}"),
        )
    })?;
    drop(host_state);
    runtime_state.service_ingress("destack.display.window.setVisibility")?;

    Ok(())
}
