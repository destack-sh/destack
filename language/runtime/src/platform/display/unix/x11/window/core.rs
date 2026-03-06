use x11rb::connection::Connection;
use x11rb::errors::ConnectionError;
use x11rb::properties::{AspectRatio as X11AspectRatio, WmSizeHints};
use x11rb::protocol::shape::{ConnectionExt as ShapeConnectionExt, SK, SO};
use x11rb::protocol::xproto::{
    AtomEnum, ClipOrdering, ConnectionExt as XprotoConnectionExt, PropMode,
};
use x11rb::wrapper::ConnectionExt as X11WrapperConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayMode, WindowAspectRatio, WindowChromeKind, WindowLogicalSize, WindowModeOptions,
    WindowOcclusionState, WindowPhysicalSize, WindowSizeConstraints, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};

use super::super::core;
use super::super::model::X11WindowBinding;
use super::constants::*;

/// Resolve one occlusion value from one x11 visibility state.
pub(crate) fn occlusion_from_visibility(visibility: WindowVisibility) -> WindowOcclusionState {
    // hidden and minimized windows are not visible to presentation
    if visibility == WindowVisibility::Hidden || visibility == WindowVisibility::Minimized {
        return WindowOcclusionState::Occluded;
    }

    // x11 does not expose one portable compositor occlusion query for visible windows
    WindowOcclusionState::Unknown
}

/// Apply one decoration policy through `_MOTIF_WM_HINTS`.
pub(crate) fn apply_window_decorated(
    connection_state: &core::X11ConnectionState,
    window: u32,
    decorated: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // encode one motif-hints payload with the decorations lane enabled
    let motif_hints = [
        MOTIF_HINTS_DECORATIONS_FLAG,
        0,
        // evaluate this condition
        if decorated { 1 } else { 0 },
        0,
        0,
    ];

    // write `_MOTIF_WM_HINTS` and flush the request stream
    connection_state
        .connection
        .change_property32(
            PropMode::REPLACE,
            window,
            connection_state.atoms.motif_wm_hints,
            connection_state.atoms.motif_wm_hints,
            &motif_hints,
        )
        .map_err(|error| core::io_error(operation, format!("change_property32 failed: {error}")))?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one window chrome kind through `_NET_WM_WINDOW_TYPE`.
pub(crate) fn apply_window_chrome(
    connection_state: &core::X11ConnectionState,
    window: u32,
    chrome: WindowChromeKind,
    operation: &'static str,
) -> RuntimeResult<()> {
    // map one chrome kind to one EWMH window-type atom
    let window_type = match chrome {
        WindowChromeKind::Standard => connection_state.atoms.net_wm_window_type_normal,
        WindowChromeKind::Tool => connection_state.atoms.net_wm_window_type_utility,
        WindowChromeKind::Popup => connection_state.atoms.net_wm_window_type_popup_menu,
    };

    // write one `_NET_WM_WINDOW_TYPE` value and flush the request stream
    connection_state
        .connection
        .change_property32(
            PropMode::REPLACE,
            window,
            connection_state.atoms.net_wm_window_type,
            AtomEnum::ATOM,
            &[window_type],
        )
        .map_err(|error| core::io_error(operation, format!("change_property32 failed: {error}")))?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one normal-hints payload for resizable, constraint, and aspect-ratio lanes.
pub(crate) fn apply_window_size_hints(
    connection_state: &core::X11ConnectionState,
    window: u32,
    resizable: bool,
    constraints: Option<WindowSizeConstraints>,
    aspect_ratio: Option<WindowAspectRatio>,
    current_size: WindowPhysicalSize,
    operation: &'static str,
) -> RuntimeResult<()> {
    // begin with one empty size-hints payload
    let mut hints = WmSizeHints::new();

    // lock min and max to the current size when the window is non-resizable
    if !resizable {
        let current_width = (current_size.width.max(1)).min(i32::MAX as u32) as i32;
        let current_height = (current_size.height.max(1)).min(i32::MAX as u32) as i32;
        hints.min_size = Some((current_width, current_height));
        hints.max_size = Some((current_width, current_height));
    }
    // otherwise apply explicit min and max constraints when present
    else if let Some(constraints) = constraints {
        // evaluate this condition
        if let Some(minimum) = constraints.min {
            let min_width = normalized_constraint_component(minimum.width);
            let min_height = normalized_constraint_component(minimum.height);
            hints.min_size = Some((min_width, min_height));
        }

        // evaluate this condition
        if let Some(maximum) = constraints.max {
            let max_width = normalized_constraint_component(maximum.width);
            let max_height = normalized_constraint_component(maximum.height);
            hints.max_size = Some((max_width, max_height));
        }
    }

    // apply one fixed aspect-ratio lane when requested
    if let Some(aspect_ratio) = aspect_ratio {
        let numerator = (aspect_ratio.numerator.max(1)).min(i32::MAX as u32) as i32;
        let denominator = (aspect_ratio.denominator.max(1)).min(i32::MAX as u32) as i32;
        let ratio = X11AspectRatio::new(numerator, denominator);
        hints.aspect = Some((ratio, ratio));
    }

    // write WM normal hints and flush request bytes
    let cookie = hints
        .set_normal_hints(connection_state.connection.as_ref(), window)
        .map_err(|error| core::io_error(operation, format!("set_normal_hints failed: {error}")))?;
    cookie.check().map_err(|error| {
        core::io_error(operation, format!("set_normal_hints check failed: {error}"))
    })?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one mouse-passthrough input-shape policy through the shape extension.
pub(crate) fn apply_window_mouse_passthrough(
    connection_state: &core::X11ConnectionState,
    window: u32,
    passthrough: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // apply one empty input region for passthrough windows
    if passthrough {
        let request = connection_state.connection.shape_rectangles(
            SO::SET,
            SK::INPUT,
            ClipOrdering::UNSORTED,
            window,
            0,
            0,
            &[],
        );
        let cookie = match request {
            Ok(cookie) => cookie,
            Err(ConnectionError::UnsupportedExtension) => {
                return Err(core_platform::not_supported(operation));
            }
            Err(error) => {
                return Err(core::io_error(
                    operation,
                    format!("shape_rectangles failed: {error}"),
                ));
            }
        };
        cookie.check().map_err(|error| {
            core::io_error(operation, format!("shape_rectangles check failed: {error}"))
        })?;
    }
    // otherwise restore default window input shape
    else {
        let request =
            connection_state
                .connection
                .shape_mask(SO::SET, SK::INPUT, window, 0, 0, 0u32);
        let cookie = match request {
            Ok(cookie) => cookie,
            Err(ConnectionError::UnsupportedExtension) => return Ok(()),
            Err(error) => {
                return Err(core::io_error(
                    operation,
                    format!("shape_mask failed: {error}"),
                ));
            }
        };
        cookie.check().map_err(|error| {
            core::io_error(operation, format!("shape_mask check failed: {error}"))
        })?;
    }

    // flush one shape mutation request
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one transient-owner relationship through `WM_TRANSIENT_FOR`.
pub(crate) fn apply_window_transient_owner(
    connection_state: &core::X11ConnectionState,
    window: u32,
    owner_window: Option<u32>,
    operation: &'static str,
) -> RuntimeResult<()> {
    // write or clear the transient-owner property
    if let Some(owner_window) = owner_window {
        connection_state
            .connection
            .change_property32(
                PropMode::REPLACE,
                window,
                AtomEnum::WM_TRANSIENT_FOR,
                AtomEnum::WINDOW,
                &[owner_window],
            )
            .map_err(|error| {
                core::io_error(operation, format!("change_property32 failed: {error}"))
            })?;
    } else {
        connection_state
            .connection
            .delete_property(window, AtomEnum::WM_TRANSIENT_FOR.into())
            .map_err(|error| {
                core::io_error(operation, format!("delete_property failed: {error}"))
            })?;
    }

    // flush one transient-owner update
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Return one normalized i32 constraint component.
fn normalized_constraint_component(value: f64) -> i32 {
    value.round().clamp(1.0, i32::MAX as f64).max(1.0) as i32
}

/// Set one window title across ICCCM and EWMH properties.
pub(crate) fn set_window_title(
    connection_state: &core::X11ConnectionState,
    window: u32,
    title: &str,
) -> RuntimeResult<()> {
    connection_state
        .connection
        .change_property8(
            PropMode::REPLACE,
            window,
            connection_state.atoms.wm_name,
            AtomEnum::STRING,
            title.as_bytes(),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setTitle",
                format!("change_property8 WM_NAME failed: {error}"),
            )
        })?;
    connection_state
        .connection
        .change_property8(
            PropMode::REPLACE,
            window,
            connection_state.atoms.net_wm_name,
            connection_state.atoms.utf8_string,
            title.as_bytes(),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setTitle",
                format!("change_property8 _NET_WM_NAME failed: {error}"),
            )
        })?;

    Ok(())
}

/// Enforce owner-thread affinity for one window binding.
pub(crate) fn ensure_window_thread(
    binding: &X11WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let current_thread_id = std::thread::current().id();
    // evaluate this condition
    if binding.owner_thread_id == current_thread_id {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        "window",
        format!("{operation} must run on the owner thread for this window"),
    ))
}
