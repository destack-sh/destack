use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceInfoVm, InputEventVm};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.input.device.close.
pub(super) fn destack_input_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.device.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.device.list.
pub(super) fn destack_input_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<InputDeviceInfoVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.device.list is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.device.open.
pub(super) fn destack_input_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::InputDeviceHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.device.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.event.read.
pub(super) fn destack_input_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.event.setGrab.
pub(super) fn destack_input_set_grab(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enable);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.setGrab is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.event.tryRead.
pub(super) fn destack_input_try_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.tryRead is not available in the VM yet",
    ))
    .boxed())
}
