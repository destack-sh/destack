use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayGammaTableCapacity, CGError, CGGammaValue,
    CGGetDisplayTransferByTable, CGSetDisplayTransferByTable,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::DisplayGammaRamp;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::{core, resource as display_resource};
use super::core as monitor_core;

/// Convert one normalized gamma sample into one ABI gamma value.
fn gamma_sample_to_u16(sample: CGGammaValue) -> u16 {
    (sample.clamp(0.0, 1.0) * (u16::MAX as f32)).round() as u16
}

/// Convert one ABI gamma entry into one CoreGraphics sample.
fn gamma_sample_from_u16(sample: u16) -> CGGammaValue {
    (sample as f32) / (u16::MAX as f32)
}

/// Read one display gamma-ramp payload from CoreGraphics.
pub(crate) fn read_display_gamma_ramp(
    display: CGDirectDisplayID,
    operation: &'static str,
) -> RuntimeResult<(Vec<u16>, Vec<u16>, Vec<u16>)> {
    let capacity = CGDisplayGammaTableCapacity(display);

    // reject displays that do not expose a gamma table
    if capacity == 0 {
        return Err(core_platform::not_supported(operation));
    }

    let capacity_usize = core_platform::u32_to_usize(capacity);
    let mut red = vec![0.0 as CGGammaValue; capacity_usize];
    let mut green = vec![0.0 as CGGammaValue; capacity_usize];
    let mut blue = vec![0.0 as CGGammaValue; capacity_usize];
    let mut sample_count = 0u32;
    let status = unsafe {
        CGGetDisplayTransferByTable(
            display,
            capacity,
            red.as_mut_ptr(),
            green.as_mut_ptr(),
            blue.as_mut_ptr(),
            &mut sample_count,
        )
    };

    // surface CoreGraphics read failures explicitly
    if status != CGError(0) {
        return Err(core::io_error(
            operation,
            format!(
                "CGGetDisplayTransferByTable failed with status {:?}",
                status
            ),
        ));
    }

    let sample_count = core_platform::u32_to_usize(sample_count);
    red.truncate(sample_count);
    green.truncate(sample_count);
    blue.truncate(sample_count);

    Ok((
        red.into_iter().map(gamma_sample_to_u16).collect(),
        green.into_iter().map(gamma_sample_to_u16).collect(),
        blue.into_iter().map(gamma_sample_to_u16).collect(),
    ))
}

/// Apply one display gamma-ramp payload through CoreGraphics.
pub(crate) fn write_display_gamma_ramp(
    display: CGDirectDisplayID,
    red: &[u16],
    green: &[u16],
    blue: &[u16],
    operation: &'static str,
) -> RuntimeResult<()> {
    let red = red
        .iter()
        .copied()
        .map(gamma_sample_from_u16)
        .collect::<Vec<_>>();
    let green = green
        .iter()
        .copied()
        .map(gamma_sample_from_u16)
        .collect::<Vec<_>>();
    let blue = blue
        .iter()
        .copied()
        .map(gamma_sample_from_u16)
        .collect::<Vec<_>>();
    let status = unsafe {
        CGSetDisplayTransferByTable(
            display,
            red.len() as u32,
            red.as_ptr(),
            green.as_ptr(),
            blue.as_ptr(),
        )
    };

    // return once CoreGraphics accepts the new gamma table
    if status == CGError(0) {
        return Ok(());
    }

    Err(core::io_error(
        operation,
        format!(
            "CGSetDisplayTransferByTable failed with status {:?}",
            status
        ),
    ))
}

/// Read display gamma ramp.
pub(crate) unsafe fn monitor_gamma_ramp(
    binding: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(binding, handle, "destack.display.monitor.gammaRamp")?;
    let display = monitor_core::display_from_id(&display_id).ok_or_else(|| {
        core_platform::invalid_argument("handle", "display id is not one AppKit display identifier")
    })?;
    let (red, green, blue) = read_display_gamma_ramp(display, "destack.display.monitor.gammaRamp")?;

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
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.setGammaRamp",
    )?;
    let display = monitor_core::display_from_id(&display_id).ok_or_else(|| {
        core_platform::invalid_argument("handle", "display id is not one AppKit display identifier")
    })?;
    let red = unsafe { ramp.red.as_slice()? };
    let green = unsafe { ramp.green.as_slice()? };
    let blue = unsafe { ramp.blue.as_slice()? };

    // reject empty gamma tables
    if red.is_empty() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must not be empty",
        ));
    }

    // require equal channel lengths
    if red.len() != green.len() || red.len() != blue.len() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must have equal lengths",
        ));
    }

    write_display_gamma_ramp(
        display,
        red,
        green,
        blue,
        "destack.display.monitor.setGammaRamp",
    )
}
