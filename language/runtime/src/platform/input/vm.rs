use destack_vm as vm;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::{VmSlice};
use crate::platform::{resource};
use crate::platform::input::{InputDeviceInfoVm, InputDeviceKind, InputEventKind, InputEventVm};
use crate::runtime::RuntimeCallContext;

/// Stub for destack.input.close.
pub(super) fn destack_input_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.list.
pub(super) fn destack_input_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<VmSlice<InputDeviceInfoVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.list is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.open.
pub(super) fn destack_input_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::InputDeviceHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.read.
pub(super) fn destack_input_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.setGrab.
pub(super) fn destack_input_set_grab(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enable);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.setGrab is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.input.tryRead.
pub(super) fn destack_input_try_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.tryRead is not available in the VM yet",
    ))
    .boxed())
}

