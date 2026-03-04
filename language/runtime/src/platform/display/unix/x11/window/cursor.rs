use x11rb::connection::Connection;
use x11rb::errors::ConnectionError;
use x11rb::protocol::xfixes::ConnectionExt as XfixesConnectionExt;
use x11rb::protocol::xproto::{
    ChangeWindowAttributesAux, ConnectionExt as XprotoConnectionExt, EventMask, GrabMode,
    GrabStatus,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowCursorIcon, WindowCursorMode, WindowPosition};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::super::model::X11WindowBinding;
use super::super::super::{core, resource as display_resource};

/// Core xcursor font name used by x11 cursor glyph lookup.
const X11_CURSOR_FONT_NAME: &[u8] = b"cursor";

/// Set cursor icon for one window.
pub(crate) unsafe fn window_set_cursor_icon(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setCursorIcon")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setCursorIcon",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setCursorIcon")?;

    // release one previously cached native cursor before switching icon kind
    if let Some(cursor_handle) = binding.cursor_handle.take() {
        free_cursor(
            connection_state.as_ref(),
            cursor_handle,
            "destack.display.window.setCursorIcon",
        )?;
    }
    binding.cursor_icon = icon;

    // re-apply cursor mode and visibility lanes after icon mutation
    apply_cursor_state(
        connection_state.as_ref(),
        &mut binding,
        "destack.display.window.setCursorIcon",
    )
}

/// Set cursor interaction mode for one window.
pub(crate) unsafe fn window_set_cursor_mode(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setCursorMode")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setCursorMode",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setCursorMode")?;
    binding.cursor_mode = mode;

    // apply host cursor behavior for the requested mode
    apply_cursor_state(
        connection_state.as_ref(),
        &mut binding,
        "destack.display.window.setCursorMode",
    )
}

/// Set cursor position for one window.
pub(crate) unsafe fn window_set_cursor_position(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve runtime and window binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setCursorPosition")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setCursorPosition",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setCursorPosition")?;

    // warp pointer into the target window coordinate space
    connection_state
        .connection
        .warp_pointer(
            0u32,
            binding.window,
            0,
            0,
            0,
            0,
            position.x as i16,
            position.y as i16,
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setCursorPosition",
                format!("warp_pointer failed: {error}"),
            )
        })?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setCursorPosition",
            format!("flush failed: {error}"),
        )
    })?;

    Ok(())
}

/// Set cursor visibility for one window.
pub(crate) unsafe fn window_set_cursor_visible(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setCursorVisible")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setCursorVisible",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setCursorVisible")?;
    binding.cursor_visible = visible;

    // apply host cursor visibility and mode lanes
    apply_cursor_state(
        connection_state.as_ref(),
        &mut binding,
        "destack.display.window.setCursorVisible",
    )
}

/// Release one cached native cursor handle from one window binding.
pub(super) fn release_window_cursor(
    connection_state: &core::X11ConnectionState,
    binding: &mut X11WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(cursor_handle) = binding.cursor_handle.take() else {
        return Ok(());
    };

    free_cursor(connection_state, cursor_handle, operation)
}

/// Apply host cursor mode and visibility state for one window binding.
fn apply_cursor_state(
    connection_state: &core::X11ConnectionState,
    binding: &mut X11WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve pointer-grab policy from the requested cursor mode
    let should_grab_pointer = matches!(
        binding.cursor_mode,
        WindowCursorMode::Locked | WindowCursorMode::Confined
    );
    // evaluate this condition
    if should_grab_pointer {
        grab_pointer(connection_state, binding.window, operation)?;
    } else {
        ungrab_pointer(connection_state, operation)?;
    }

    // resolve pointer-visibility policy from mode and explicit visibility state
    let should_hide_pointer = !binding.cursor_visible
        || matches!(
            binding.cursor_mode,
            WindowCursorMode::Hidden | WindowCursorMode::Locked
        );
    set_pointer_hidden(
        connection_state,
        binding.window,
        should_hide_pointer,
        operation,
    )?;

    // apply one icon cursor while pointer visibility is enabled
    if !should_hide_pointer {
        apply_icon_cursor(connection_state, binding, operation)?;
    }

    // center the pointer while entering locked mode
    if binding.cursor_mode == WindowCursorMode::Locked {
        let center_x = (binding.size_physical.width / 2).min(i16::MAX as u32) as i16;
        let center_y = (binding.size_physical.height / 2).min(i16::MAX as u32) as i16;
        connection_state
            .connection
            .warp_pointer(0u32, binding.window, 0, 0, 0, 0, center_x, center_y)
            .map_err(|error| core::io_error(operation, format!("warp_pointer failed: {error}")))?;
    }

    // flush one cursor-update batch
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one icon cursor to one x11 window binding.
fn apply_icon_cursor(
    connection_state: &core::X11ConnectionState,
    binding: &mut X11WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    // apply default host cursor and release cached custom cursor state
    if binding.cursor_icon == WindowCursorIcon::Default {
        connection_state
            .connection
            .change_window_attributes(binding.window, &ChangeWindowAttributesAux::new().cursor(0))
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("change_window_attributes failed: {error}"),
                )
            })?;
        // evaluate this condition
        if let Some(cursor_handle) = binding.cursor_handle.take() {
            free_cursor(connection_state, cursor_handle, operation)?;
        }

        return Ok(());
    }

    // lazily allocate one native cursor for the requested icon lane
    if binding.cursor_handle.is_none() {
        let cursor_handle = create_cursor(connection_state, binding.cursor_icon, operation)?;
        binding.cursor_handle = Some(cursor_handle);
    }
    let cursor_handle = binding.cursor_handle.unwrap_or(0);

    // apply one custom cursor handle through window attributes
    connection_state
        .connection
        .change_window_attributes(
            binding.window,
            &ChangeWindowAttributesAux::new().cursor(cursor_handle),
        )
        .map_err(|error| {
            core::io_error(
                operation,
                format!("change_window_attributes failed: {error}"),
            )
        })?;

    Ok(())
}

/// Create one native cursor handle from one icon selector.
fn create_cursor(
    connection_state: &core::X11ConnectionState,
    icon: WindowCursorIcon,
    operation: &'static str,
) -> RuntimeResult<u32> {
    // allocate one cursor-font id and open the xcursor font resource
    let cursor_font = connection_state.connection.generate_id().map_err(|error| {
        core::io_error(
            operation,
            format!("generate_id for cursor font failed: {error}"),
        )
    })?;
    let open_cookie = connection_state
        .connection
        .open_font(cursor_font, X11_CURSOR_FONT_NAME)
        .map_err(|error| core::io_error(operation, format!("open_font failed: {error}")))?;
    open_cookie
        .check()
        .map_err(|error| core::io_error(operation, format!("open_font check failed: {error}")))?;

    // allocate one cursor id and create one glyph-backed cursor
    let cursor_handle = connection_state.connection.generate_id().map_err(|error| {
        core::io_error(
            operation,
            format!("generate_id for cursor handle failed: {error}"),
        )
    })?;
    let glyph = cursor_icon_glyph(icon);
    let create_cookie = connection_state
        .connection
        .create_glyph_cursor(
            cursor_handle,
            cursor_font,
            cursor_font,
            glyph,
            glyph.saturating_add(1),
            0,
            0,
            0,
            u16::MAX,
            u16::MAX,
            u16::MAX,
        )
        .map_err(|error| {
            core::io_error(operation, format!("create_glyph_cursor failed: {error}"))
        })?;
    create_cookie.check().map_err(|error| {
        core::io_error(
            operation,
            format!("create_glyph_cursor check failed: {error}"),
        )
    })?;

    // release the cursor-font resource after cursor creation
    let close_cookie = connection_state
        .connection
        .close_font(cursor_font)
        .map_err(|error| core::io_error(operation, format!("close_font failed: {error}")))?;
    close_cookie
        .check()
        .map_err(|error| core::io_error(operation, format!("close_font check failed: {error}")))?;

    Ok(cursor_handle)
}

/// Release one native x11 cursor handle.
fn free_cursor(
    connection_state: &core::X11ConnectionState,
    cursor_handle: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let cookie = connection_state
        .connection
        .free_cursor(cursor_handle)
        .map_err(|error| core::io_error(operation, format!("free_cursor failed: {error}")))?;
    cookie
        .check()
        .map_err(|error| core::io_error(operation, format!("free_cursor check failed: {error}")))?;

    Ok(())
}

/// Apply one xfixes cursor visibility request.
fn set_pointer_hidden(
    connection_state: &core::X11ConnectionState,
    window: u32,
    hidden: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // hide cursor through xfixes when one hidden policy is requested
    if hidden {
        let request = connection_state.connection.xfixes_hide_cursor(window);
        let cookie = match request {
            Ok(cookie) => cookie,
            Err(ConnectionError::UnsupportedExtension) => {
                return Err(core_platform::not_supported(operation));
            }
            Err(error) => {
                return Err(core::io_error(
                    operation,
                    format!("xfixes_hide_cursor failed: {error}"),
                ));
            }
        };
        cookie.check().map_err(|error| {
            core::io_error(
                operation,
                format!("xfixes_hide_cursor check failed: {error}"),
            )
        })?;

        return Ok(());
    }

    // show cursor through xfixes when available
    let request = connection_state.connection.xfixes_show_cursor(window);
    let cookie = match request {
        Ok(cookie) => cookie,
        Err(ConnectionError::UnsupportedExtension) => return Ok(()),
        Err(error) => {
            return Err(core::io_error(
                operation,
                format!("xfixes_show_cursor failed: {error}"),
            ));
        }
    };
    cookie.check().map_err(|error| {
        core::io_error(
            operation,
            format!("xfixes_show_cursor check failed: {error}"),
        )
    })?;

    Ok(())
}

/// Apply one pointer grab for lock or confine modes.
fn grab_pointer(
    connection_state: &core::X11ConnectionState,
    window: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let reply = connection_state
        .connection
        .grab_pointer(
            false,
            window,
            EventMask::POINTER_MOTION | EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            window,
            0u32,
            x11rb::CURRENT_TIME,
        )
        .map_err(|error| {
            core::io_error(operation, format!("grab_pointer request failed: {error}"))
        })?
        .reply()
        .map_err(|error| {
            core::io_error(operation, format!("grab_pointer reply failed: {error}"))
        })?;
    // evaluate this condition
    if reply.status != GrabStatus::SUCCESS {
        return Err(core_platform::io_busy(
            operation,
            format!("grab_pointer failed with status {:?}", reply.status),
        ));
    }

    Ok(())
}

/// Release one active pointer grab.
fn ungrab_pointer(
    connection_state: &core::X11ConnectionState,
    operation: &'static str,
) -> RuntimeResult<()> {
    connection_state
        .connection
        .ungrab_pointer(x11rb::CURRENT_TIME)
        .map_err(|error| core::io_error(operation, format!("ungrab_pointer failed: {error}")))?;

    Ok(())
}

/// Return one cursor-font glyph code for one icon selector.
fn cursor_icon_glyph(icon: WindowCursorIcon) -> u16 {
    // resolve this variant
    match icon {
        WindowCursorIcon::Default => 68,
        WindowCursorIcon::Text => 152,
        WindowCursorIcon::Crosshair => 34,
        WindowCursorIcon::Pointer => 60,
        WindowCursorIcon::Help => 92,
        WindowCursorIcon::Wait => 150,
        WindowCursorIcon::Progress => 150,
        WindowCursorIcon::Move => 52,
        WindowCursorIcon::NotAllowed => 0,
        WindowCursorIcon::Grab => 58,
        WindowCursorIcon::Grabbing => 60,
        WindowCursorIcon::EResize => 96,
        WindowCursorIcon::NResize => 138,
        WindowCursorIcon::NeResize => 136,
        WindowCursorIcon::NwResize => 134,
        WindowCursorIcon::SResize => 16,
        WindowCursorIcon::SeResize => 14,
        WindowCursorIcon::SwResize => 12,
        WindowCursorIcon::WResize => 70,
        WindowCursorIcon::EwResize => 108,
        WindowCursorIcon::NsResize => 116,
        WindowCursorIcon::NeswResize => 12,
        WindowCursorIcon::NwseResize => 14,
        WindowCursorIcon::ColResize => 108,
        WindowCursorIcon::RowResize => 116,
        WindowCursorIcon::AllScroll => 52,
        WindowCursorIcon::ZoomIn => 92,
        WindowCursorIcon::ZoomOut => 92,
    }
}
