use super::{core as input_core, raw as raw_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceKind, InputTouchState};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened touch-capable device descriptor.
fn resolve_touch_device(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<raw_input::RawInputDeviceDescriptor> {
    // resolve one opened raw device descriptor for this handle
    let device = input_core::raw_device(binding, handle, operation)?;

    // accept touch or pen-class devices for touch snapshots
    if device.kind != InputDeviceKind::Touch && device.kind != InputDeviceKind::Pen {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(device)
}

/// Read one touch state snapshot.
pub(crate) unsafe fn destack_input_touch_state(
    binding: &BindingCallContext,
    out: *mut InputTouchState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one touch-capable opened device descriptor
    let device = resolve_touch_device(binding, handle, "destack.input.touch.state")?;

    // read one current touch snapshot from active contact state
    let snapshot =
        raw_input::read_touch_state_snapshot(binding, &device, "destack.input.touch.state")?;

    // write one decoded snapshot payload
    unsafe {
        *out = snapshot;
    }

    Ok(())
}
