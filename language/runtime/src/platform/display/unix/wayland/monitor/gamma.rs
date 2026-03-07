use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, FromRawFd};
use std::sync::{Arc, Mutex};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayColorSpace, DisplayColorState, DisplayGammaRamp, DisplayHdrMode,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use crate::platform::display::unix::wayland::{core, resource as display_resource};

/// Wayland color management primaries value for sRGB.
const COLOR_PRIMARIES_SRGB: u32 = 1;
/// Wayland color management primaries value for BT.2020.
const COLOR_PRIMARIES_BT2020: u32 = 6;
/// Wayland color management primaries value for Display P3.
const COLOR_PRIMARIES_DISPLAY_P3: u32 = 9;
/// Wayland color management transfer function value for linear.
const COLOR_TRANSFER_EXT_LINEAR: u32 = 5;
/// Wayland color management transfer function value for PQ.
const COLOR_TRANSFER_ST2084_PQ: u32 = 11;
/// Wayland color management transfer function value for HLG.
const COLOR_TRANSFER_HLG: u32 = 13;
/// Wayland image description failure cause value for low interface version.
const COLOR_FAILURE_CAUSE_LOW_VERSION: u32 = 0;
/// Wayland image description failure cause value for unsupported description.
const COLOR_FAILURE_CAUSE_UNSUPPORTED: u32 = 1;
/// Wayland image description failure cause value for compositor internal failure.
const COLOR_FAILURE_CAUSE_OPERATING_SYSTEM: u32 = 2;
/// Wayland image description failure cause value for removed output.
const COLOR_FAILURE_CAUSE_NO_OUTPUT: u32 = 3;

/// Return one wl_output global name for one stable display id.
fn output_global_name_for_display_id(display_id: &str) -> RuntimeResult<u32> {
    core::output_global_name_from_display_id(display_id).ok_or_else(|| {
        core_platform::invalid_argument(
            "display",
            format!("display id `{display_id}` is not one wayland output identifier"),
        )
    })
}

/// Return one color description query snapshot for one display id.
fn color_query_for_display_id(
    context: &BindingCallContext,
    display_id: &str,
    operation: &'static str,
) -> RuntimeResult<core::WaylandColorDescriptionQueryState> {
    core::with_connection_dispatch(
        context,
        operation,
        |_connection, event_queue, dispatch_state| {
            // resolve one color management output object for this display
            let output_global_name = output_global_name_for_display_id(display_id)?;
            let output = dispatch_state
                .output
                .color_outputs_by_global
                .get(&output_global_name)
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;

            // request one image description snapshot from the compositor
            let query_state = Arc::new(Mutex::new(
                core::WaylandColorDescriptionQueryState::default(),
            ));
            let image_description =
                output.get_image_description(&event_queue.handle(), Arc::clone(&query_state));
            core::flush_queue(event_queue, operation)?;

            // wait until image description is ready or failed
            for _ in 0..6 {
                let snapshot = query_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone();
                if snapshot.ready || snapshot.failed_message.is_some() {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        operation,
                        format!("wayland image description roundtrip failed: {error}"),
                    )
                })?;
            }

            let snapshot = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if let Some(message) = snapshot.failed_message {
                let cause = snapshot.failed_cause.unwrap_or_default();
                if cause == COLOR_FAILURE_CAUSE_LOW_VERSION
                    || cause == COLOR_FAILURE_CAUSE_UNSUPPORTED
                    || cause == COLOR_FAILURE_CAUSE_NO_OUTPUT
                {
                    return Err(core_platform::not_supported(operation));
                }

                if cause == COLOR_FAILURE_CAUSE_OPERATING_SYSTEM {
                    return Err(core_platform::io_operation_error(
                        operation,
                        None,
                        format!("wayland output image description failed: {message}"),
                    ));
                }

                return Err(core::io_error(
                    operation,
                    format!("wayland output image description failed: {message}"),
                ));
            }
            if !snapshot.ready {
                return Err(core_platform::io_would_block(
                    operation,
                    "timed out waiting for wayland output image description",
                ));
            }

            // request one information payload for this ready image description
            image_description.get_information(&event_queue.handle(), Arc::clone(&query_state));
            core::flush_queue(event_queue, operation)?;

            // wait until information delivery is complete
            for _ in 0..6 {
                let snapshot = query_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone();
                if snapshot.info_done || snapshot.failed_message.is_some() {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        operation,
                        format!("wayland image description info roundtrip failed: {error}"),
                    )
                })?;
            }

            // destroy the temporary image description object before returning
            image_description.destroy();
            core::flush_queue(event_queue, operation)?;

            let snapshot = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if let Some(message) = snapshot.failed_message {
                let cause = snapshot.failed_cause.unwrap_or_default();
                if cause == COLOR_FAILURE_CAUSE_LOW_VERSION
                    || cause == COLOR_FAILURE_CAUSE_UNSUPPORTED
                    || cause == COLOR_FAILURE_CAUSE_NO_OUTPUT
                {
                    return Err(core_platform::not_supported(operation));
                }

                if cause == COLOR_FAILURE_CAUSE_OPERATING_SYSTEM {
                    return Err(core_platform::io_operation_error(
                        operation,
                        None,
                        format!("wayland output image description failed: {message}"),
                    ));
                }

                return Err(core::io_error(
                    operation,
                    format!("wayland output image description failed: {message}"),
                ));
            }
            if !snapshot.info_done {
                return Err(core_platform::io_would_block(
                    operation,
                    "timed out waiting for wayland output image description details",
                ));
            }

            Ok(snapshot)
        },
    )
}

/// Return one backend color space value from one color description query.
fn color_space_from_query(query: &core::WaylandColorDescriptionQueryState) -> DisplayColorSpace {
    // classify explicit BT.2020 plus PQ as HDR10 output signaling
    if query.primaries_named == Some(COLOR_PRIMARIES_BT2020)
        && query.transfer_function_named == Some(COLOR_TRANSFER_ST2084_PQ)
    {
        return DisplayColorSpace::Hdr10;
    }

    // classify Display P3 primaries directly
    if query.primaries_named == Some(COLOR_PRIMARIES_DISPLAY_P3) {
        return DisplayColorSpace::DisplayP3;
    }

    // classify BT.2020 primaries without PQ signaling
    if query.primaries_named == Some(COLOR_PRIMARIES_BT2020) {
        return DisplayColorSpace::Bt2020;
    }

    // classify extended linear sRGB as scRGB style transfer
    if query.primaries_named == Some(COLOR_PRIMARIES_SRGB)
        && query.transfer_function_named == Some(COLOR_TRANSFER_EXT_LINEAR)
    {
        return DisplayColorSpace::ScRgb;
    }

    // classify explicit sRGB primaries
    if query.primaries_named == Some(COLOR_PRIMARIES_SRGB) {
        return DisplayColorSpace::Srgb;
    }

    // default to unknown when compositor does not expose named primaries
    DisplayColorSpace::Unknown
}

/// Return one backend HDR mode value from one color description query.
fn hdr_mode_from_query(query: &core::WaylandColorDescriptionQueryState) -> DisplayHdrMode {
    let is_hdr_transfer = query.transfer_function_named == Some(COLOR_TRANSFER_ST2084_PQ)
        || query.transfer_function_named == Some(COLOR_TRANSFER_HLG);
    if is_hdr_transfer {
        return DisplayHdrMode::Hdr;
    }

    let max_luminance = query.maximum_luminance.unwrap_or_default();
    if max_luminance > 300 {
        return DisplayHdrMode::Hdr;
    }

    if query.primaries_named.is_some() || query.transfer_function_named.is_some() {
        return DisplayHdrMode::Sdr;
    }

    DisplayHdrMode::Unknown
}

/// Return one gamma control query snapshot for one display id.
fn gamma_query_for_display_id(
    context: &BindingCallContext,
    display_id: &str,
    operation: &'static str,
) -> RuntimeResult<core::WaylandGammaControlQueryState> {
    core::with_connection_dispatch(
        context,
        operation,
        |_connection, event_queue, dispatch_state| {
            // resolve one wl_output endpoint and one gamma control manager
            let output_global_name = output_global_name_for_display_id(display_id)?;
            let output = dispatch_state
                .output
                .outputs_by_global
                .get(&output_global_name)
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;
            let manager = dispatch_state
                .globals
                .gamma_control_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;

            // create one temporary gamma control object
            let query_state = Arc::new(Mutex::new(core::WaylandGammaControlQueryState::default()));
            let gamma_control =
                manager.get_gamma_control(&output, &event_queue.handle(), Arc::clone(&query_state));
            core::flush_queue(event_queue, operation)?;

            // wait until gamma size or failure arrives
            for _ in 0..6 {
                let snapshot = query_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone();
                if snapshot.gamma_size.is_some() || snapshot.failed {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        operation,
                        format!("wayland gamma control roundtrip failed: {error}"),
                    )
                })?;
            }

            // destroy the temporary gamma control object before returning
            gamma_control.destroy();
            core::flush_queue(event_queue, operation)?;

            let snapshot = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if snapshot.failed {
                return Err(core_platform::not_supported(operation));
            }
            if snapshot.gamma_size.is_none() {
                return Err(core_platform::io_would_block(
                    operation,
                    "timed out waiting for wayland gamma size",
                ));
            }

            Ok(snapshot)
        },
    )
}

/// Validate one gamma ramp payload and decode copied channels.
fn decoded_gamma_ramp(ramp: DisplayGammaRamp) -> RuntimeResult<(Vec<u16>, Vec<u16>, Vec<u16>)> {
    // decode gamma channel arrays from the native payload
    let red = unsafe { ramp.red.as_slice()?.to_vec() };
    let green = unsafe { ramp.green.as_slice()?.to_vec() };
    let blue = unsafe { ramp.blue.as_slice()?.to_vec() };

    // reject empty channel payloads
    if red.is_empty() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must not be empty",
        ));
    }

    // reject channel size mismatches
    if red.len() != green.len() || red.len() != blue.len() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must have equal lengths",
        ));
    }

    // reject channels that cannot fit one wayland gamma size value
    if red.len() > u32::MAX as usize {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels are too large for this backend",
        ));
    }

    Ok((red, green, blue))
}

/// Create one anonymous in memory file descriptor for one gamma table payload.
fn create_gamma_memfd_file(operation: &'static str, length: usize) -> RuntimeResult<File> {
    // build one stable memfd label for diagnostics
    let name = CString::new("destack-wayland-gamma")
        .map_err(|error| core::io_error(operation, format!("invalid memfd name: {error}")))?;

    // reject lengths that cannot fit one off_t value
    if length > i64::MAX as usize {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma table payload is too large",
        ));
    }

    // allocate one anonymous file descriptor
    let file_descriptor = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if file_descriptor < 0 {
        let error = std::io::Error::last_os_error();
        return Err(core::io_error(
            operation,
            format!("memfd_create failed: {error}"),
        ));
    }

    // grow file to requested payload length
    let truncated = unsafe { libc::ftruncate(file_descriptor, length as libc::off_t) };
    if truncated != 0 {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::close(file_descriptor);
        }
        return Err(core::io_error(
            operation,
            format!("ftruncate failed: {error}"),
        ));
    }

    // transfer ownership to one rust file handle
    Ok(unsafe { File::from_raw_fd(file_descriptor) })
}

/// Encode one gamma ramp payload into one fd backed byte stream.
fn encoded_gamma_bytes(red: &[u16], green: &[u16], blue: &[u16]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(red.len().saturating_mul(6));

    for value in red.iter().chain(green.iter()).chain(blue.iter()) {
        encoded.extend_from_slice(&value.to_ne_bytes());
    }

    encoded
}

/// Read display color state.
pub(crate) unsafe fn monitor_color_state(
    context: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate output pointer and resolve stable display id
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.colorState",
    )?;

    // resolve one protocol backed output color snapshot
    let query = color_query_for_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.colorState",
    )?;
    let state = DisplayColorState {
        hdr_mode: hdr_mode_from_query(&query),
        color_space: color_space_from_query(&query),
        bits_per_channel: None,
    };

    unsafe {
        *out = state;
    }

    Ok(())
}

/// Read display HDR mode.
pub(crate) unsafe fn monitor_hdr_mode(
    context: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate output pointer and resolve stable display id
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.hdrMode")?;

    // resolve one protocol backed output HDR snapshot
    let query = color_query_for_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.hdrMode",
    )?;
    let mode = hdr_mode_from_query(&query);

    unsafe {
        *out = mode;
    }

    Ok(())
}

/// Set display HDR mode.
pub(crate) unsafe fn monitor_set_hdr_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    // validate display handle and mode payload
    let display_id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.setHdrMode",
    )?;
    if mode == DisplayHdrMode::Unknown {
        return Err(core_platform::invalid_argument(
            "mode",
            "hdr mode must be one concrete mode",
        ));
    }

    // allow host default policy as one no op request
    if mode == DisplayHdrMode::System {
        return Ok(());
    }

    // accept idempotent requests when the compositor already matches this mode
    let query = color_query_for_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.setHdrMode",
    )?;
    let current_mode = hdr_mode_from_query(&query);
    if current_mode == mode {
        return Ok(());
    }

    // wayland has no generic output policy write lane for HDR mode switching
    Err(core_platform::not_supported(
        "destack.display.monitor.setHdrMode",
    ))
}

/// Read display gamma ramp.
pub(crate) unsafe fn monitor_gamma_ramp(
    context: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate output pointer and resolve stable display id
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.gammaRamp")?;

    // read one cached ramp snapshot when this runtime already set gamma values
    let runtime_state = core::runtime_state(context);
    let cached = runtime_state
        .gamma_ramps_by_display_id
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(display_id.as_str())
        .cloned();
    if cached.is_none() {
        gamma_query_for_display_id(
            context,
            display_id.as_str(),
            "destack.display.monitor.gammaRamp",
        )?;
        return Err(core_platform::not_supported(
            "destack.display.monitor.gammaRamp",
        ));
    }

    // resolve one cached snapshot after explicit presence check
    let Some(snapshot) = cached else {
        return Err(core_platform::invalid_state(
            "missing cached gamma ramp snapshot",
        ));
    };

    unsafe {
        *out = DisplayGammaRamp {
            red: context.store_slice(snapshot.red),
            green: context.store_slice(snapshot.green),
            blue: context.store_slice(snapshot.blue),
        };
    }

    Ok(())
}

/// Set display gamma ramp.
pub(crate) unsafe fn monitor_set_gamma_ramp(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    // resolve stable display id and decode one copied ramp payload
    let display_id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.setGammaRamp",
    )?;
    let (red, green, blue) = decoded_gamma_ramp(ramp)?;

    // resolve expected channel length from compositor gamma size response
    let query = gamma_query_for_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.setGammaRamp",
    )?;
    let expected = usize::try_from(query.gamma_size.unwrap_or_default())
        .map_err(|_| core_platform::invalid_state("wayland gamma size overflow"))?;
    if expected == 0 {
        return Err(core_platform::not_supported(
            "destack.display.monitor.setGammaRamp",
        ));
    }
    if red.len() != expected {
        return Err(core_platform::invalid_argument(
            "ramp",
            format!("gamma ramp channels must each contain exactly {expected} entries"),
        ));
    }

    // encode this ramp payload once for fd backed upload
    let encoded = encoded_gamma_bytes(&red, &green, &blue);

    // apply gamma table through one short lived gamma control object
    core::with_connection_dispatch(
        context,
        "destack.display.monitor.setGammaRamp",
        |_connection, event_queue, dispatch_state| {
            let output_global_name = output_global_name_for_display_id(display_id.as_str())?;
            let output = dispatch_state
                .output
                .outputs_by_global
                .get(&output_global_name)
                .cloned()
                .ok_or_else(|| {
                    core_platform::not_supported("destack.display.monitor.setGammaRamp")
                })?;
            let manager = dispatch_state
                .globals
                .gamma_control_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| {
                    core_platform::not_supported("destack.display.monitor.setGammaRamp")
                })?;

            let query_state = Arc::new(Mutex::new(core::WaylandGammaControlQueryState::default()));
            let gamma_control =
                manager.get_gamma_control(&output, &event_queue.handle(), Arc::clone(&query_state));
            core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;

            // wait until one gamma size event confirms control readiness
            for _ in 0..6 {
                let snapshot = query_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone();
                if snapshot.gamma_size.is_some() || snapshot.failed {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        "destack.display.monitor.setGammaRamp",
                        format!("wayland gamma control roundtrip failed: {error}"),
                    )
                })?;
            }

            let snapshot = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if snapshot.failed {
                gamma_control.destroy();
                core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;
                return Err(core_platform::not_supported(
                    "destack.display.monitor.setGammaRamp",
                ));
            }
            if snapshot.gamma_size != Some(expected as u32) {
                gamma_control.destroy();
                core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;
                return Err(core_platform::io_operation_error(
                    "destack.display.monitor.setGammaRamp",
                    None,
                    "wayland gamma size changed while applying gamma ramp",
                ));
            }

            // upload one packed gamma table through one memfd payload
            let mut file =
                create_gamma_memfd_file("destack.display.monitor.setGammaRamp", encoded.len())?;
            file.write_all(encoded.as_slice()).map_err(|error| {
                core::io_error(
                    "destack.display.monitor.setGammaRamp",
                    format!("gamma table write failed: {error}"),
                )
            })?;
            file.flush().map_err(|error| {
                core::io_error(
                    "destack.display.monitor.setGammaRamp",
                    format!("gamma table flush failed: {error}"),
                )
            })?;
            gamma_control.set_gamma(file.as_fd());
            core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;

            // roundtrip once so failed events are visible before returning
            event_queue.roundtrip(dispatch_state).map_err(|error| {
                core::io_error(
                    "destack.display.monitor.setGammaRamp",
                    format!("wayland gamma control apply roundtrip failed: {error}"),
                )
            })?;
            let failed = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .failed;

            gamma_control.destroy();
            core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;

            if failed {
                return Err(core_platform::io_operation_error(
                    "destack.display.monitor.setGammaRamp",
                    None,
                    "compositor rejected wayland gamma table update",
                ));
            }

            Ok(())
        },
    )?;

    // cache the last applied ramp for deterministic readback behavior
    let runtime_state = core::runtime_state(context);
    let mut cache = runtime_state
        .gamma_ramps_by_display_id
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    cache.insert(
        display_id,
        core::WaylandGammaRampSnapshot { red, green, blue },
    );

    Ok(())
}
