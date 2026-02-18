use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceInfo, InputDeviceInfoVm, InputEvent, InputEventVm, InputReadMode, host as host_input,
};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, VmArray, VmSlice, resource};
use crate::runtime::RuntimeCallContext;

/// Invoke one host call that writes through an out pointer.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate one uninitialized output slot for the host call
    let mut out = std::mem::MaybeUninit::<T>::uninit();

    // execute call and assume initialization on success
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Convert one VM string handle into one runtime native string reference.
fn string_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    // resolve VM string payload and copy into runtime storage
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(runtime.store_string(value.as_str()))
}

/// Convert one native device-info payload into its VM shape.
fn device_info_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputDeviceInfo,
) -> RuntimeResult<InputDeviceInfoVm> {
    // decode borrowed native strings for vm interning
    let id = unsafe { value.id.as_str()? };
    let name = unsafe { value.name.as_str()? };

    // build one vm device-info record
    Ok(InputDeviceInfoVm {
        id: vm::StringHandle::new(context.intern_string(id)),
        name: vm::StringHandle::new(context.intern_string(name)),
        kind: value.kind,
        vendor_id: value.vendor_id,
        product_id: value.product_id,
        key_count: value.key_count,
        button_count: value.button_count,
        axis_count: value.axis_count,
        connected: value.connected,
        supports_grab: value.supports_grab,
        supports_raw: value.supports_raw,
        supports_text: value.supports_text,
        supports_rumble: value.supports_rumble,
    })
}

/// Convert one native input event payload into its VM shape.
fn event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: InputEvent,
) -> RuntimeResult<InputEventVm> {
    // decode borrowed native strings for vm interning
    let device_id = unsafe { value.device_id.as_str()? };
    let text = unsafe { value.text.as_str()? };

    // build one vm event record
    Ok(InputEventVm {
        kind: value.kind,
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        device_id: vm::StringHandle::new(context.intern_string(device_id)),
        action: value.action,
        code: value.code,
        scan_code: value.scan_code,
        value: value.value,
        x: value.x,
        y: value.y,
        wheel_x: value.wheel_x,
        wheel_y: value.wheel_y,
        modifiers: value.modifiers,
        repeat: value.repeat,
        text: vm::StringHandle::new(context.intern_string(text)),
    })
}

/// Convert one native device-info slice into one VM slice.
fn list_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<InputDeviceInfo>,
) -> RuntimeResult<VmSlice<InputDeviceInfoVm>> {
    // decode native slice and convert each item
    let values = unsafe { values.as_slice()? };
    let mut vm_values = Vec::with_capacity(values.len());
    for value in values {
        vm_values.push(device_info_to_vm(context, *value)?);
    }
    VmSlice::from_values(context, &vm_values)
}

/// Convert one native event array into one VM array.
fn event_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeArray<InputEvent>,
) -> RuntimeResult<VmArray<InputEventVm>> {
    // decode native array and convert each item
    let values = unsafe { values.as_slice()? };
    let mut vm_values = Vec::with_capacity(values.len());
    for value in values {
        vm_values.push(event_to_vm(context, *value)?);
    }
    VmArray::from_values(context, &vm_values)
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
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one discoverable input backend.
/// Uses evdev device-node enumeration on Linux, global-session and terminal discovery on macOS, terminal input discovery on other Unix hosts, and console plus raw-state discovery on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
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
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one openable input backend.
/// Uses evdev device-node open on Linux, global-session or terminal-device open on macOS, terminal-device open on other Unix hosts, and duplicated console-input handles or raw-state handles on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
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

/// Close one global input event monitor.
///
/// Close one opened monitor stream and release host subscription resources.
/// Pending unread monitor events are discarded.
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
pub(crate) fn destack_input_monitor_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_monitor_close(runtime, handle) }
}

/// Open one global input event monitor.
///
/// Open one monitor stream that reports host input topology events, including connect and disconnect.
/// Monitor streams are independent from per-device data streams.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses inotify-backed `/dev/input` monitor events on Linux with snapshot fallback when watcher setup is unavailable, global-session subscriptions on macOS, terminal-device scans on other Unix hosts, and raw-input device-change subscriptions on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_monitor_open(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::InputMonitorHandle> {
    call_out(|out| unsafe { host_input::destack_input_monitor_open(runtime, out) })
}

/// Read one global input monitor event.
///
/// Read one pending monitor event from the global input monitor stream.
/// This stream is the canonical source for device connect and disconnect events.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses blocking reads from inotify-backed Linux monitor queues with snapshot fallback on watcherless hosts, terminal or session monitor streams on Unix hosts, and raw-input monitor queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_monitor_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<InputEventVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_monitor_read(runtime, out, handle) })?;
    event_to_vm(context, value)
}

/// Poll one global input monitor event without blocking.
///
/// Poll one pending monitor event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses nonblocking reads from inotify-backed Linux monitor queues with snapshot fallback on watcherless hosts, terminal or session monitor streams on Unix hosts, and raw-input monitor queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_monitor_try_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<InputEventVm> {
    let value = call_out(|out| unsafe {
        host_input::destack_input_monitor_try_read(runtime, out, handle)
    })?;
    event_to_vm(context, value)
}

/// Read one input event.
///
/// Read one pending input event from one opened device stream.
/// Per-device streams report control and motion events for that device and exclude global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses evdev event reads on Linux, global-session state polling on macOS, terminal-byte event reads on other Unix hosts, and ReadConsoleInputW queue reads or raw-state polling on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let value = call_out(|out| unsafe { host_input::destack_input_read(runtime, out, handle) })?;
    event_to_vm(context, value)
}

/// Read one batch of input events.
///
/// Read up to `maxEvents` events from one opened device stream in one call.
/// Batch ordering matches backend delivery order and excludes global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses batched reads when supported and runtime looped reads otherwise.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_read_batch(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<InputEventVm>> {
    let values = call_out(|out| unsafe {
        host_input::destack_input_read_batch(runtime, out, handle, maxevents)
    })?;
    event_array_to_vm(context, values)
}

/// Enable or disable exclusive device grab.
///
/// Toggle exclusive-grab mode for one input device when the host backend supports it.
/// Grabs can prevent event delivery to other clients.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where exclusive grab is not defined by host policy.
/// Uses EVIOCGRAB on Linux, returns notSupported for global-session and terminal-backed Unix input, and uses SetConsoleMode capture toggles on Windows console input.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
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

/// Select event decoding mode for one input stream.
///
/// Select translated or raw decoding mode for one opened input endpoint.
/// Hosts can return notSupported when raw mode is unavailable for the selected endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses per-stream runtime mode selection on Linux evdev and macOS session backends, termios raw and cooked mode updates on Unix TTY paths, and SetConsoleMode updates on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_set_read_mode(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    unsafe { host_input::destack_input_set_read_mode(runtime, handle, mode) }
}

/// Poll one input event without blocking.
///
/// Poll one pending input event from one opened device stream and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses nonblocking evdev reads on Linux, nonblocking global-session polling on macOS, nonblocking terminal-byte reads on other Unix hosts, and nonblocking console queue reads or raw-state polling on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_input_try_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<InputEventVm> {
    let value =
        call_out(|out| unsafe { host_input::destack_input_try_read(runtime, out, handle) })?;
    event_to_vm(context, value)
}
