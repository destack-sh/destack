#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::device::{
    MidiBackendDescriptor, MidiEvent, MidiEventSubscriptionOptions, MidiInputPortOpenOptions,
    MidiInputRecord, MidiOutputPortOpenOptions, MidiOutputRecord, MidiPortDescriptor,
    MidiPortListOptions, MidiVirtualInputCreateOptions, MidiVirtualOutputCreateOptions,
};
use crate::platform::{core, resource};

/// List host MIDI backends.
pub(crate) unsafe fn destack_device_midi_backend_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiBackendDescriptor>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.backend.list",
    ))
    .boxed())
}

/// Close one MIDI topology event subscription.
pub(crate) unsafe fn destack_device_midi_event_close(
    _binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.event.close",
    ))
    .boxed())
}

/// Open one MIDI topology event subscription.
pub(crate) unsafe fn destack_device_midi_event_open(
    _binding: &BindingCallContext,
    out: *mut resource::MidiEventHandle,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.event.open",
    ))
    .boxed())
}

/// Wait for one MIDI topology event.
pub(crate) unsafe fn destack_device_midi_event_read(
    _binding: &BindingCallContext,
    out: *mut MidiEvent,
    handle: resource::MidiEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.event.read",
    ))
    .boxed())
}

/// Wait for one batch of MIDI topology events.
pub(crate) unsafe fn destack_device_midi_event_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiEvent>,
    handle: resource::MidiEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxevents, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.event.readBatch",
    ))
    .boxed())
}

/// Poll one MIDI topology event without blocking.
pub(crate) unsafe fn destack_device_midi_event_try_read(
    _binding: &BindingCallContext,
    out: *mut MidiEvent,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.event.tryRead",
    ))
    .boxed())
}

/// Poll one batch of MIDI topology events without blocking.
pub(crate) unsafe fn destack_device_midi_event_try_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiEvent>,
    handle: resource::MidiEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.event.tryReadBatch",
    ))
    .boxed())
}

/// Close one opened MIDI input endpoint.
pub(crate) unsafe fn destack_device_midi_input_port_close(
    _binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.port.close",
    ))
    .boxed())
}

/// List available MIDI input endpoints.
pub(crate) unsafe fn destack_device_midi_input_port_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiPortDescriptor>,
    options: MidiPortListOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.port.list",
    ))
    .boxed())
}

/// Open one MIDI input endpoint.
pub(crate) unsafe fn destack_device_midi_input_port_open(
    _binding: &BindingCallContext,
    out: *mut resource::MidiInputPortHandle,
    id: NativeStringRef,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, id, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.port.open",
    ))
    .boxed())
}

/// Describe one opened MIDI input endpoint.
pub(crate) unsafe fn destack_device_midi_input_port_descriptor(
    _binding: &BindingCallContext,
    out: *mut MidiPortDescriptor,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.port.descriptor",
    ))
    .boxed())
}

/// Read one MIDI input record.
pub(crate) unsafe fn destack_device_midi_input_read(
    _binding: &BindingCallContext,
    out: *mut MidiInputRecord,
    handle: resource::MidiInputPortHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.read",
    ))
    .boxed())
}

/// Read one batch of MIDI input records.
pub(crate) unsafe fn destack_device_midi_input_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeArray<MidiInputRecord>,
    handle: resource::MidiInputPortHandle,
    maxrecords: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxrecords, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.readBatch",
    ))
    .boxed())
}

/// Poll one MIDI input record without blocking.
pub(crate) unsafe fn destack_device_midi_input_try_read(
    _binding: &BindingCallContext,
    out: *mut MidiInputRecord,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.tryRead",
    ))
    .boxed())
}

/// Poll one batch of MIDI input records without blocking.
pub(crate) unsafe fn destack_device_midi_input_try_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeArray<MidiInputRecord>,
    handle: resource::MidiInputPortHandle,
    maxrecords: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxrecords);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.tryReadBatch",
    ))
    .boxed())
}

/// Create one virtual MIDI input endpoint.
pub(crate) unsafe fn destack_device_midi_input_virtual_create(
    _binding: &BindingCallContext,
    out: *mut resource::MidiInputPortHandle,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.input.virtual.create",
    ))
    .boxed())
}

/// Close one opened MIDI output endpoint.
pub(crate) unsafe fn destack_device_midi_output_port_close(
    _binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.output.port.close",
    ))
    .boxed())
}

/// List available MIDI output endpoints.
pub(crate) unsafe fn destack_device_midi_output_port_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiPortDescriptor>,
    options: MidiPortListOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.output.port.list",
    ))
    .boxed())
}

/// Open one MIDI output endpoint.
pub(crate) unsafe fn destack_device_midi_output_port_open(
    _binding: &BindingCallContext,
    out: *mut resource::MidiOutputPortHandle,
    id: NativeStringRef,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, id, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.output.port.open",
    ))
    .boxed())
}

/// Describe one opened MIDI output endpoint.
pub(crate) unsafe fn destack_device_midi_output_port_descriptor(
    _binding: &BindingCallContext,
    out: *mut MidiPortDescriptor,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.output.port.descriptor",
    ))
    .boxed())
}

/// Create one virtual MIDI output endpoint.
pub(crate) unsafe fn destack_device_midi_output_virtual_create(
    _binding: &BindingCallContext,
    out: *mut resource::MidiOutputPortHandle,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.output.virtual.create",
    ))
    .boxed())
}

/// Write one batch of outbound MIDI records.
pub(crate) unsafe fn destack_device_midi_output_write(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::MidiOutputPortHandle,
    records: NativeArray<MidiOutputRecord>,
) -> RuntimeResult<()> {
    let _ = (out, handle, records);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.midi.output.write",
    ))
    .boxed())
}
