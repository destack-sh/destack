use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceInfo, InputDeviceInfoVm, InputEventVm, host as host_input,
};
use crate::platform::{NativeSlice, NativeStringRef, VmSlice, resource};
use crate::runtime::RuntimeCallContext;

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

fn string_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    Ok(runtime.store_string(value.as_str()))
}

fn list_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<InputDeviceInfo>,
) -> RuntimeResult<VmSlice<InputDeviceInfoVm>> {
    let values = unsafe { values.as_slice()? };
    let mut vm_values = Vec::with_capacity(values.len());
    for value in values {
        let id = unsafe { value.id.as_str()? };
        let name = unsafe { value.name.as_str()? };
        let item = InputDeviceInfoVm {
            id: vm::StringHandle::new(context.intern_string(id)),
            name: vm::StringHandle::new(context.intern_string(name)),
            kind: value.kind,
            vendor_id: value.vendor_id,
            product_id: value.product_id,
            connected: value.connected,
        };

        let field_0 = item.id.value();
        let field_1 = item.name.value();
        let field_2 = vm::Value::uint(item.kind as u8 as u64, 8);
        let field_3 = vm::Value::uint(item.vendor_id as u64, 16);
        let field_4 = vm::Value::uint(item.product_id as u64, 16);
        let field_5 = vm::Value::bool(item.connected);
        vm_values.push(
            context.allocate_aggregate(vec![field_0, field_1, field_2, field_3, field_4, field_5]),
        );
    }

    Ok(VmSlice {
        data: context.allocate_raw_values(vm_values),
        len: values.len() as u32,
        _marker: std::marker::PhantomData,
    })
}

/// Close one input device.
///
/// Close one opened input device endpoint and release host resources.
/// Pending unread events are discarded according to host backend behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_close(runtime, handle) }
}

/// List available input devices.
///
/// Enumerate host input devices and return stable identifiers and typed device metadata.
/// Device ordering and hotplug visibility follow host input subsystem semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses evdev device-node enumeration on Linux, terminal input discovery on other Unix hosts, and console-input availability checks on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_list(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<InputDeviceInfoVm>> {
    let values = call_out(|out| unsafe { host_input::destack_input_list(runtime, out) })?;
    list_to_vm(context, values)
}

/// Open one input device.
///
/// Open one input device endpoint for event reads and optional control operations.
/// Exclusive-grab behavior and permission checks are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses evdev device-node open on Linux, terminal-device open on other Unix hosts, and duplicated console-input handles on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_open(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::InputDeviceHandle> {
    let id = string_from_vm(runtime, context, id)?;
    call_out(|out| unsafe { host_input::destack_input_open(runtime, out, id) })
}

/// Read one input event.
///
/// Read one pending input event from the runtime input queue.
/// Event ordering follows host event queue delivery semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses evdev event reads on Linux, terminal-byte event reads on other Unix hosts, and ReadConsoleInputW queue reads on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_read(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    call_out(|out| unsafe { host_input::destack_input_read(runtime, out, handle) })
}

/// Enable or disable exclusive device grab.
///
/// Toggle exclusive-grab mode for one input device when the host backend supports it.
/// Grabs can prevent event delivery to other clients.
///
/// # Platform
/// Unix and Windows.
/// Uses EVIOCGRAB on Linux, returns notSupported for terminal-backed Unix input, and uses SetConsoleMode capture toggles on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_set_grab(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_set_grab(runtime, handle, enable) }
}

/// Poll one input event without blocking.
///
/// Poll one pending input event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking evdev reads on Linux, nonblocking terminal-byte reads on other Unix hosts, and nonblocking console queue reads on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_try_read(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    call_out(|out| unsafe { host_input::destack_input_try_read(runtime, out, handle) })
}
