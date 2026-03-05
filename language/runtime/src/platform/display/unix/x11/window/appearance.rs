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
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    always_on_top: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate resolved_binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setAlwaysOnTop")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.setAlwaysOnTop")?;
    resolved_binding.always_on_top = always_on_top;
    super::set_net_wm_state(
        connection_state.as_ref(),
        resolved_binding.window,
        connection_state.atoms.net_wm_state_above,
        always_on_top,
    )?;

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate resolved_binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setDecorated")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setDecorated",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.setDecorated")?;

    // apply one host decoration mutation before updating the snapshot
    super::apply_window_decorated(
        connection_state.as_ref(),
        resolved_binding.window,
        decorated,
        "destack.display.window.setDecorated",
    )?;
    resolved_binding.decorated = decorated;

    Ok(())
}

/// Set window resizable state.
pub(crate) unsafe fn window_set_resizable(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate resolved_binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setResizable")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setResizable",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.setResizable")?;

    // apply one host size-hints mutation before updating the snapshot
    super::apply_window_size_hints(
        connection_state.as_ref(),
        resolved_binding.window,
        resizable,
        resolved_binding.constraints,
        resolved_binding.aspect_ratio,
        resolved_binding.size_physical,
        "destack.display.window.setResizable",
    )?;
    resolved_binding.resizable = resizable;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    // resolve runtime and mutate resolved_binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setChrome")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setChrome",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.setChrome")?;

    // apply one host window-type mutation before updating the snapshot
    super::apply_window_chrome(
        connection_state.as_ref(),
        resolved_binding.window,
        chrome,
        "destack.display.window.setChrome",
    )?;
    resolved_binding.chrome = chrome;

    Ok(())
}

/// Set one window icon set.
pub(crate) unsafe fn window_set_icons(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    // resolve runtime and window resolved_binding lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setIcons")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setIcons",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.setIcons")?;

    // apply icon property update or clear it when no icons are configured
    if let Some(icons) = icons {
        let payload = net_wm_icon_payload(icons)?;
        connection_state
            .connection
            .change_property32(
                PropMode::REPLACE,
                resolved_binding.window,
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
            .delete_property(resolved_binding.window, connection_state.atoms.net_wm_icon)
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
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate resolved_binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setMousePassthrough")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(
        &resolved_binding,
        "destack.display.window.setMousePassthrough",
    )?;

    // apply one host input-shape mutation before updating the snapshot
    super::apply_window_mouse_passthrough(
        connection_state.as_ref(),
        resolved_binding.window,
        passthrough,
        "destack.display.window.setMousePassthrough",
    )?;
    resolved_binding.mouse_passthrough = passthrough;

    Ok(())
}

/// Set one window opacity.
pub(crate) unsafe fn window_set_opacity(
    binding: &BindingCallContext,
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

    // resolve runtime and mutate resolved_binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setOpacity")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setOpacity",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.setOpacity")?;
    resolved_binding.opacity = opacity;

    // apply EWMH opacity property on the window
    let encoded = (opacity.clamp(0.0, 1.0) * (u32::MAX as f64)).round() as u32;
    connection_state
        .connection
        .change_property32(
            PropMode::REPLACE,
            resolved_binding.window,
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
    binding: &BindingCallContext,
    out: *mut f64,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer and read resolved_binding state
    core_platform::ensure_out(out, "out")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.opacity",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.opacity")?;
    unsafe {
        *out = resolved_binding.opacity;
    }

    Ok(())
}

/// Set one window taskbar visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate resolved_binding state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setTaskbarVisible")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setTaskbarVisible",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(
        &resolved_binding,
        "destack.display.window.setTaskbarVisible",
    )?;
    resolved_binding.taskbar_visible = visible;
    super::set_net_wm_state(
        connection_state.as_ref(),
        resolved_binding.window,
        connection_state.atoms.net_wm_state_skip_taskbar,
        !visible,
    )?;

    Ok(())
}

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    // parse title payload and resolve runtime and resolved_binding state
    let title = unsafe { title.as_str()? };
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setTitle")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setTitle",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.setTitle")?;

    // apply host title updates and mutate resolved_binding snapshot
    super::set_window_title(connection_state.as_ref(), resolved_binding.window, title)?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setTitle",
            format!("flush failed: {error}"),
        )
    })?;
    resolved_binding.title = title.to_string();

    Ok(())
}

/// Set one window visibility state.
pub(crate) unsafe fn window_set_visibility(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    // resolve runtime and resolved_binding lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setVisibility")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.setVisibility",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.setVisibility")?;

    // apply host visibility transitions
    let previous_visibility = resolved_binding.visibility;
    // resolve this variant
    match visibility {
        WindowVisibility::Hidden => {
            connection_state
                .connection
                .unmap_window(resolved_binding.window)
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
                .map_window(resolved_binding.window)
                .map_err(|error| {
                    core::io_error(
                        "destack.display.window.setVisibility",
                        format!("map_window failed: {error}"),
                    )
                })?;
        }
        WindowVisibility::Minimized => {
            connection_state
                .connection
                .unmap_window(resolved_binding.window)
                .map_err(|error| {
                    core::io_error(
                        "destack.display.window.setVisibility",
                        format!("unmap_window failed: {error}"),
                    )
                })?;
        }
    }
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setVisibility",
            format!("flush failed: {error}"),
        )
    })?;
    resolved_binding.visibility = visibility;
    drop(resolved_binding);

    event::publish_window_visibility_changed(
        &runtime_state,
        window_handle,
        previous_visibility,
        visibility,
    );

    Ok(())
}
