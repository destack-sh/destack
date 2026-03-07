use x11rb::connection::Connection;
use x11rb::protocol::randr::{ConnectionExt as RandrConnectionExt, Crtc};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayColorSpace, DisplayColorState, DisplayGammaRamp, DisplayHdrMode,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::X11_DEFAULT_BITS_PER_CHANNEL;
use super::mode::resolve_output_state_by_display_id;
use crate::platform::display::unix::x11::{core, resource as display_resource};

/// Resolve one active randr CRTC for gamma and color operations.
fn resolve_randr_crtc(
    connection_state: &core::X11ConnectionState,
    operation: &'static str,
) -> RuntimeResult<Option<Crtc>> {
    // read current randr screen resources for this root window
    let resources = connection_state
        .connection
        .randr_get_screen_resources_current(connection_state.root)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_screen_resources_current request failed: {error}"),
            )
        })?
        .reply()
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_screen_resources_current reply failed: {error}"),
            )
        })?;
    if resources.crtcs.is_empty() {
        return Ok(None);
    }

    // prefer the primary output CRTC when available
    let primary_output = connection_state
        .connection
        .randr_get_output_primary(connection_state.root)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_output_primary request failed: {error}"),
            )
        })?
        .reply()
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_output_primary reply failed: {error}"),
            )
        })?;
    if primary_output.output != 0 {
        let output_info = connection_state
            .connection
            .randr_get_output_info(primary_output.output, resources.config_timestamp)
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_get_output_info request failed: {error}"),
                )
            })?
            .reply()
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_get_output_info reply failed: {error}"),
                )
            })?;
        if output_info.crtc != 0 {
            return Ok(Some(output_info.crtc));
        }
    }

    // otherwise use the first connected output that has an active CRTC
    for output in resources.outputs.iter().copied() {
        let output_info = connection_state
            .connection
            .randr_get_output_info(output, resources.config_timestamp)
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_get_output_info request failed: {error}"),
                )
            })?
            .reply()
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_get_output_info reply failed: {error}"),
                )
            })?;
        if output_info.crtc != 0 {
            return Ok(Some(output_info.crtc));
        }
    }

    // fall back to the first advertised CRTC lane
    Ok(resources.crtcs.iter().copied().find(|crtc| *crtc != 0))
}

/// Resolve one active randr CRTC for one display id.
fn resolve_randr_crtc_for_display(
    connection_state: &core::X11ConnectionState,
    display_id: &str,
    operation: &'static str,
) -> RuntimeResult<Option<Crtc>> {
    // prefer the monitor-specific output when one randr-backed id is available
    if let Some(state) =
        resolve_output_state_by_display_id(connection_state, display_id, operation)?
    {
        return Ok(Some(state.crtc));
    }

    resolve_randr_crtc(connection_state, operation)
}

/// Read one randr gamma size for one CRTC.
fn read_randr_gamma_size(
    connection_state: &core::X11ConnectionState,
    crtc: Crtc,
    operation: &'static str,
) -> RuntimeResult<usize> {
    let reply = connection_state
        .connection
        .randr_get_crtc_gamma_size(crtc)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_crtc_gamma_size request failed: {error}"),
            )
        })?
        .reply()
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_crtc_gamma_size reply failed: {error}"),
            )
        })?;

    Ok(reply.size as usize)
}

/// Read one randr gamma ramp payload for one CRTC.
fn read_randr_gamma_ramp(
    connection_state: &core::X11ConnectionState,
    crtc: Crtc,
    operation: &'static str,
) -> RuntimeResult<(Vec<u16>, Vec<u16>, Vec<u16>)> {
    let reply = connection_state
        .connection
        .randr_get_crtc_gamma(crtc)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_crtc_gamma request failed: {error}"),
            )
        })?
        .reply()
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_crtc_gamma reply failed: {error}"),
            )
        })?;
    if reply.red.is_empty()
        || reply.red.len() != reply.green.len()
        || reply.red.len() != reply.blue.len()
    {
        return Err(core::io_error(
            operation,
            "randr_get_crtc_gamma returned invalid channel lengths",
        ));
    }

    Ok((reply.red, reply.green, reply.blue))
}

/// Write one randr gamma ramp payload for one CRTC.
fn write_randr_gamma_ramp(
    connection_state: &core::X11ConnectionState,
    crtc: Crtc,
    red: &[u16],
    green: &[u16],
    blue: &[u16],
    operation: &'static str,
) -> RuntimeResult<()> {
    let cookie = connection_state
        .connection
        .randr_set_crtc_gamma(crtc, red, green, blue)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_set_crtc_gamma request failed: {error}"),
            )
        })?;
    cookie.check().map_err(|error| {
        core::io_error(
            operation,
            format!("randr_set_crtc_gamma check failed: {error}"),
        )
    })?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Read display color state.
pub(crate) unsafe fn monitor_color_state(
    binding: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and display handle
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.colorState",
    )?;
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.monitor.colorState")?;
    let crtc = resolve_randr_crtc_for_display(
        connection_state.as_ref(),
        &display_id,
        "destack.display.monitor.colorState",
    )?
    .ok_or_else(|| core_platform::not_supported("destack.display.monitor.colorState"))?;

    // require one randr gamma lane for color state support
    let gamma_size = read_randr_gamma_size(
        connection_state.as_ref(),
        crtc,
        "destack.display.monitor.colorState",
    )?;
    if gamma_size == 0 {
        return Err(core_platform::not_supported(
            "destack.display.monitor.colorState",
        ));
    }

    let state = DisplayColorState {
        hdr_mode: DisplayHdrMode::Unknown,
        color_space: DisplayColorSpace::Srgb,
        bits_per_channel: Some(X11_DEFAULT_BITS_PER_CHANNEL),
    };

    unsafe {
        *out = state;
    }

    Ok(())
}

/// Read display HDR mode.
pub(crate) unsafe fn monitor_hdr_mode(
    binding: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and display handle
    core_platform::ensure_out(out, "out")?;
    display_resource::resolve_display_id(binding, handle, "destack.display.monitor.hdrMode")?;

    unsafe {
        *out = DisplayHdrMode::Unknown;
    }

    Ok(())
}

/// Set display HDR mode.
pub(crate) unsafe fn monitor_set_hdr_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    // validate display handle and mode payload
    display_resource::resolve_display_id(binding, handle, "destack.display.monitor.setHdrMode")?;
    if mode == DisplayHdrMode::Unknown {
        return Err(core_platform::invalid_argument(
            "mode",
            "hdr mode must be one concrete mode",
        ));
    }

    // x11 has no portable hdr control primitive
    if mode == DisplayHdrMode::System {
        return Ok(());
    }
    if mode == DisplayHdrMode::Hdr || mode == DisplayHdrMode::Sdr {
        return Err(core_platform::not_supported(
            "destack.display.monitor.setHdrMode",
        ));
    }

    Ok(())
}

/// Read display gamma ramp.
pub(crate) unsafe fn monitor_gamma_ramp(
    binding: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and display handle
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(binding, handle, "destack.display.monitor.gammaRamp")?;
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.monitor.gammaRamp")?;
    let crtc = resolve_randr_crtc_for_display(
        connection_state.as_ref(),
        &display_id,
        "destack.display.monitor.gammaRamp",
    )?
    .ok_or_else(|| core_platform::not_supported("destack.display.monitor.gammaRamp"))?;

    // read and encode the current gamma channels
    let (red, green, blue) = read_randr_gamma_ramp(
        connection_state.as_ref(),
        crtc,
        "destack.display.monitor.gammaRamp",
    )?;
    unsafe {
        *out = DisplayGammaRamp {
            red: binding.store_slice(red),
            green: binding.store_slice(green),
            blue: binding.store_slice(blue),
        };
    }

    Ok(())
}

/// Set display gamma ramp.
pub(crate) unsafe fn monitor_set_gamma_ramp(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    // validate display handle and decode channel payloads
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.setGammaRamp",
    )?;
    let red = unsafe { ramp.red.as_slice()? };
    let green = unsafe { ramp.green.as_slice()? };
    let blue = unsafe { ramp.blue.as_slice()? };
    if red.is_empty() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must not be empty",
        ));
    }
    if red.len() != green.len() || red.len() != blue.len() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must have equal lengths",
        ));
    }

    // resolve one active crtc and validate gamma table size
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.monitor.setGammaRamp")?;
    let crtc = resolve_randr_crtc_for_display(
        connection_state.as_ref(),
        &display_id,
        "destack.display.monitor.setGammaRamp",
    )?
    .ok_or_else(|| core_platform::not_supported("destack.display.monitor.setGammaRamp"))?;
    let expected = read_randr_gamma_size(
        connection_state.as_ref(),
        crtc,
        "destack.display.monitor.setGammaRamp",
    )?;
    if red.len() != expected {
        return Err(core_platform::invalid_argument(
            "ramp",
            format!("gamma ramp channels must each contain exactly {expected} entries"),
        ));
    }

    // write the new gamma ramp through randr
    write_randr_gamma_ramp(
        connection_state.as_ref(),
        crtc,
        red,
        green,
        blue,
        "destack.display.monitor.setGammaRamp",
    )
}
