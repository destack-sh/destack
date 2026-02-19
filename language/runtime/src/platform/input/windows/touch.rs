use super::{core as input_core, raw as raw_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceKind, InputTouchState};
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Resolve one opened touch-capable device descriptor.
fn resolve_touch_device(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<raw_input::RawInputDeviceDescriptor> {
    // resolve one opened raw device descriptor for this handle
    let device = input_core::raw_device(context, handle, operation)?;

    // accept touch or pen-class devices for touch snapshots
    if device.kind != InputDeviceKind::Touch && device.kind != InputDeviceKind::Pen {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(device)
}

/// Read one touch state snapshot.
///
/// Return one current touch-contact snapshot for one opened touch-capable device.
/// Contact ordering follows backend delivery order.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where touch snapshots are unavailable.
/// Uses backend-specific contact tables from evdev or libinput style paths on Unix and pointer-contact APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_touch_state(
    context: &RuntimeCallContext,
    out: *mut InputTouchState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one touch-capable opened device descriptor
    let device = resolve_touch_device(context, handle, "destack.input.touch.state")?;

    // read one current touch snapshot from active contact state
    let snapshot =
        raw_input::read_touch_state_snapshot(context, &device, "destack.input.touch.state")?;

    // write one decoded snapshot payload
    unsafe {
        *out = snapshot;
    }

    Ok(())
}
