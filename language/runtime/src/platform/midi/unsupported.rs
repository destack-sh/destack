#![allow(dead_code)]
#![cfg_attr(any(target_os = "macos", windows), allow(dead_code))]

use crate::diagnostic::RuntimeResult;
use crate::platform::midi::core::{
    MidiBackendDescriptorValue, MidiEventValue, MidiInputRecordValue, MidiOutputRecordValue,
    MidiPortDescriptorValue,
};
use crate::platform::midi::{
    MidiEventSubscriptionOptions, MidiInputPortOpenOptions, MidiOutputPortOpenOptions,
    MidiPortListOptions, MidiVirtualInputCreateOptions, MidiVirtualOutputCreateOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// List MIDI backends on unsupported hosts.
pub(crate) fn midi_backend_list(
    _binding: &BindingCallContext,
) -> RuntimeResult<Vec<MidiBackendDescriptorValue>> {
    Err(core_platform::not_supported("destack.midi.backend.list"))
}

/// List MIDI input ports on unsupported hosts.
pub(crate) fn midi_input_port_list(
    _binding: &BindingCallContext,
    _options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    Err(core_platform::not_supported("destack.midi.input.port.list"))
}

/// Open one MIDI input port on unsupported hosts.
pub(crate) fn midi_input_port_open(
    _binding: &BindingCallContext,
    _id: &str,
    _options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    Err(core_platform::not_supported("destack.midi.input.port.open"))
}

/// Describe one MIDI input port on unsupported hosts.
pub(crate) fn midi_input_port_descriptor(
    _binding: &BindingCallContext,
    _handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    Err(core_platform::not_supported(
        "destack.midi.input.port.descriptor",
    ))
}

/// Close one MIDI input port on unsupported hosts.
pub(crate) fn midi_input_port_close(
    _binding: &BindingCallContext,
    _handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported(
        "destack.midi.input.port.close",
    ))
}

/// Read one MIDI input record on unsupported hosts.
pub(crate) fn midi_input_read(
    _binding: &BindingCallContext,
    _handle: resource::MidiInputPortHandle,
    _timeout_ns: u64,
) -> RuntimeResult<MidiInputRecordValue> {
    Err(core_platform::not_supported("destack.midi.input.read"))
}

/// Read one MIDI input record batch on unsupported hosts.
pub(crate) fn midi_input_read_batch(
    _binding: &BindingCallContext,
    _handle: resource::MidiInputPortHandle,
    _max_records: u32,
    _timeout_ns: u64,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    Err(core_platform::not_supported("destack.midi.input.readBatch"))
}

/// Poll one MIDI input record on unsupported hosts.
pub(crate) fn midi_input_try_read(
    _binding: &BindingCallContext,
    _handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    Err(core_platform::not_supported("destack.midi.input.tryRead"))
}

/// Poll one MIDI input record batch on unsupported hosts.
pub(crate) fn midi_input_try_read_batch(
    _binding: &BindingCallContext,
    _handle: resource::MidiInputPortHandle,
    _max_records: u32,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    Err(core_platform::not_supported(
        "destack.midi.input.tryReadBatch",
    ))
}

/// Create one virtual MIDI input on unsupported hosts.
pub(crate) fn midi_input_virtual_create(
    _binding: &BindingCallContext,
    _options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    Err(core_platform::not_supported(
        "destack.midi.input.virtual.create",
    ))
}

/// List MIDI output ports on unsupported hosts.
pub(crate) fn midi_output_port_list(
    _binding: &BindingCallContext,
    _options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    Err(core_platform::not_supported(
        "destack.midi.output.port.list",
    ))
}

/// Open one MIDI output port on unsupported hosts.
pub(crate) fn midi_output_port_open(
    _binding: &BindingCallContext,
    _id: &str,
    _options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    Err(core_platform::not_supported(
        "destack.midi.output.port.open",
    ))
}

/// Describe one MIDI output port on unsupported hosts.
pub(crate) fn midi_output_port_descriptor(
    _binding: &BindingCallContext,
    _handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    Err(core_platform::not_supported(
        "destack.midi.output.port.descriptor",
    ))
}

/// Close one MIDI output port on unsupported hosts.
pub(crate) fn midi_output_port_close(
    _binding: &BindingCallContext,
    _handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported(
        "destack.midi.output.port.close",
    ))
}

/// Write MIDI output records on unsupported hosts.
pub(crate) fn midi_output_write(
    _binding: &BindingCallContext,
    _handle: resource::MidiOutputPortHandle,
    _records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    Err(core_platform::not_supported("destack.midi.output.write"))
}

/// Flush queued MIDI output records on unsupported hosts.
pub(crate) fn midi_output_flush(
    _binding: &BindingCallContext,
    _handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported("destack.midi.output.flush"))
}

/// Create one virtual MIDI output on unsupported hosts.
pub(crate) fn midi_output_virtual_create(
    _binding: &BindingCallContext,
    _options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    Err(core_platform::not_supported(
        "destack.midi.output.virtual.create",
    ))
}

/// Open one MIDI event subscription on unsupported hosts.
pub(crate) fn midi_event_open(
    _binding: &BindingCallContext,
    _options: MidiEventSubscriptionOptions,
) -> RuntimeResult<resource::MidiEventHandle> {
    Err(core_platform::not_supported("destack.midi.event.open"))
}

/// Close one MIDI event subscription on unsupported hosts.
pub(crate) fn midi_event_close(
    _binding: &BindingCallContext,
    _handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported("destack.midi.event.close"))
}

/// Read one MIDI event on unsupported hosts.
pub(crate) fn midi_event_read(
    _binding: &BindingCallContext,
    _handle: resource::MidiEventHandle,
    _timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    Err(core_platform::not_supported("destack.midi.event.read"))
}

/// Read one MIDI event batch on unsupported hosts.
pub(crate) fn midi_event_read_batch(
    _binding: &BindingCallContext,
    _handle: resource::MidiEventHandle,
    _max_events: u32,
    _timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    Err(core_platform::not_supported("destack.midi.event.readBatch"))
}

/// Poll one MIDI event on unsupported hosts.
pub(crate) fn midi_event_try_read(
    _binding: &BindingCallContext,
    _handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    Err(core_platform::not_supported("destack.midi.event.tryRead"))
}

/// Poll one MIDI event batch on unsupported hosts.
pub(crate) fn midi_event_try_read_batch(
    _binding: &BindingCallContext,
    _handle: resource::MidiEventHandle,
    _max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    Err(core_platform::not_supported(
        "destack.midi.event.tryReadBatch",
    ))
}
