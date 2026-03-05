use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as XprotoConnectionExt, PropMode};
use x11rb::wrapper::ConnectionExt as X11WrapperConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{WindowChromeKind, WindowIconSet, WindowVisibility};
use crate::platform::resource::WindowHandle;
use crate::runtime::{BindingCallContext, NativeStringRef};

use super::super::super::{core, event, resource as display_resource};
use super::icon::net_wm_icon_payload;

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    always_on_top: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setAlwaysOnTop")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setAlwaysOnTop")?;
    binding.always_on_top = always_on_top;
    super::set_net_wm_state(
        connection_state.as_ref(),
        binding.window,
        connection_state.atoms.net_wm_state_above,
        always_on_top,
    )?;

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setDecorated")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setDecorated",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setDecorated")?;

    // apply one host decoration mutation before updating the snapshot
    super::apply_window_decorated(
        connection_state.as_ref(),
        binding.window,
        decorated,
        "destack.display.window.setDecorated",
    )?;
    binding.decorated = decorated;

    Ok(())
}

/// Set window resizable state.
pub(crate) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setResizable")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setResizable",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setResizable")?;

    // apply one host size-hints mutation before updating the snapshot
    super::apply_window_size_hints(
        connection_state.as_ref(),
        binding.window,
        resizable,
        binding.constraints,
        binding.aspect_ratio,
        binding.size_physical,
        "destack.display.window.setResizable",
    )?;
    binding.resizable = resizable;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setChrome")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setChrome",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setChrome")?;

    // apply one host window-type mutation before updating the snapshot
    super::apply_window_chrome(
        connection_state.as_ref(),
        binding.window,
        chrome,
        "destack.display.window.setChrome",
    )?;
    binding.chrome = chrome;

    Ok(())
}

/// Set one window icon set.
pub(crate) unsafe fn window_set_icons(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    // resolve runtime and window binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setIcons")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setIcons",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setIcons")?;

    // apply icon property update or clear it when no icons are configured
    if let Some(icons) = icons {
        let payload = net_wm_icon_payload(icons)?;
        connection_state
            .connection
            .change_property32(
                PropMode::REPLACE,
                binding.window,
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
            .delete_property(binding.window, connection_state.atoms.net_wm_icon)
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
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setMousePassthrough")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setMousePassthrough")?;

    // apply one host input-shape mutation before updating the snapshot
    super::apply_window_mouse_passthrough(
        connection_state.as_ref(),
        binding.window,
        passthrough,
        "destack.display.window.setMousePassthrough",
    )?;
    binding.mouse_passthrough = passthrough;

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

    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setOpacity")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setOpacity",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setOpacity")?;
    binding.opacity = opacity;

    // apply EWMH opacity property on the window
    let encoded = (opacity.clamp(0.0, 1.0) * (u32::MAX as f64)).round() as u32;
    connection_state
        .connection
        .change_property32(
            PropMode::REPLACE,
            binding.window,
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

    Ok(())
}

/// Read one window opacity.
pub(crate) unsafe fn window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer and read binding state
    core_platform::ensure_out(out, "out")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.opacity",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.opacity")?;
    unsafe {
        *out = binding.opacity;
    }

    Ok(())
}

/// Set one window taskbar visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setTaskbarVisible")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setTaskbarVisible",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setTaskbarVisible")?;
    binding.taskbar_visible = visible;
    super::set_net_wm_state(
        connection_state.as_ref(),
        binding.window,
        connection_state.atoms.net_wm_state_skip_taskbar,
        !visible,
    )?;

    Ok(())
}

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    // parse title payload and resolve runtime and binding state
    let title = unsafe { title.as_str()? };
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setTitle")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setTitle",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setTitle")?;

    // apply host title updates and mutate binding snapshot
    super::set_window_title(connection_state.as_ref(), binding.window, title)?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setTitle",
            format!("flush failed: {error}"),
        )
    })?;
    binding.title = title.to_string();

    Ok(())
}

/// Set one window visibility state.
pub(crate) unsafe fn window_set_visibility(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    // resolve runtime and binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setVisibility")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setVisibility",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setVisibility")?;

    // apply host visibility transitions
    let previous_visibility = binding.visibility;
    // resolve this variant
    match visibility {
        WindowVisibility::Hidden => {
            connection_state
                .connection
                .unmap_window(binding.window)
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
                .map_window(binding.window)
                .map_err(|error| {
                    core::io_error(
                        "destack.display.window.setVisibility",
                        format!("map_window failed: {error}"),
                    )
                })?;
        }
        WindowVisibility::Minimized => {
            super::request_window_minimize(
                connection_state.as_ref(),
                binding.window,
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
    binding.visibility = visibility;
    drop(binding);

    event::publish_window_visibility_changed(
        &runtime_state,
        window_handle,
        previous_visibility,
        visibility,
    );

    Ok(())
}
