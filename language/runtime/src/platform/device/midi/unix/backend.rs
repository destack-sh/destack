use crate::diagnostic::RuntimeResult;
use crate::platform::core::BackendSupport;
use crate::platform::device::midi::core::{
    MidiBackendDescriptorValue, MidiEventValue, MidiInputRecordValue, MidiOutputRecordValue,
    MidiPortDescriptorValue, midi_backend_descriptors, midi_selector_support,
    require_midi_event_handle_backend, require_midi_input_handle_backend,
    require_midi_output_handle_backend, resolve_midi_backend_selection,
};
use crate::platform::device::{
    MidiBackend, MidiBackendSelectionPolicy, MidiEventSubscriptionOptions,
    MidiInputPortOpenOptions, MidiOutputPortOpenOptions, MidiPortListOptions,
    MidiVirtualInputCreateOptions, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::{alsa, jack};

/// Host backend selectors considered by auto selection on Linux.
const PREFERRED_HOST_BACKENDS: [MidiBackend; 2] = [MidiBackend::Alsa, MidiBackend::JackMidi];

/// Return host support for one backend selector on Linux.
pub(super) fn backend_support(backend: MidiBackend) -> BackendSupport {
    midi_selector_support(&PREFERRED_HOST_BACKENDS, backend, |backend| match backend {
        MidiBackend::Alsa => Some(alsa::backend_support(MidiBackend::Alsa)),
        MidiBackend::JackMidi => Some(jack::backend_support()),
        _ => None,
    })
}

/// Resolve one effective Linux host backend.
fn resolve_backend(
    backend: MidiBackend,
    policy: MidiBackendSelectionPolicy,
    operation: &'static str,
) -> RuntimeResult<MidiBackend> {
    resolve_midi_backend_selection(
        &PREFERRED_HOST_BACKENDS,
        backend,
        policy,
        operation,
        backend_support,
    )
}

/// Build backend descriptors for Linux MIDI hosts.
pub(crate) fn midi_backend_list(
    _binding: &BindingCallContext,
) -> RuntimeResult<Vec<MidiBackendDescriptorValue>> {
    Ok(midi_backend_descriptors(
        &PREFERRED_HOST_BACKENDS,
        backend_support,
        |backend, support| match backend {
            MidiBackend::Alsa => alsa::backend_metadata(MidiBackend::Alsa, support),
            MidiBackend::JackMidi => jack::backend_metadata(support),
            _ => Default::default(),
        },
    ))
}

/// List Linux MIDI input endpoints.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.input.port.list",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_port_list(binding, options),
        MidiBackend::JackMidi => jack::midi_input_port_list(binding, options),
        _ => unreachable!("linux MIDI backend resolution must return one host backend"),
    }
}

/// Open one Linux MIDI input endpoint.
pub(crate) fn midi_input_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.input.port.open",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_port_open(binding, id, options),
        MidiBackend::JackMidi => jack::midi_input_port_open(binding, id, options),
        _ => unreachable!("linux MIDI backend resolution must return one host backend"),
    }
}

/// Describe one opened Linux MIDI input endpoint.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let backend = require_midi_input_handle_backend(
        binding,
        handle.0,
        "destack.device.midi.input.port.descriptor",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_port_descriptor(binding, handle),
        MidiBackend::JackMidi => jack::midi_input_port_descriptor(binding, handle),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Close one opened Linux MIDI input endpoint.
pub(crate) fn midi_input_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let backend = require_midi_input_handle_backend(
        binding,
        handle.0,
        "destack.device.midi.input.port.close",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_port_close(binding, handle),
        MidiBackend::JackMidi => jack::midi_input_port_close(binding, handle),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Wait for one Linux MIDI input record.
pub(crate) fn midi_input_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiInputRecordValue> {
    let backend =
        require_midi_input_handle_backend(binding, handle.0, "destack.device.midi.input.read")?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_read(binding, handle, timeout_ns),
        MidiBackend::JackMidi => jack::midi_input_read(binding, handle, timeout_ns),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Wait for one batch of Linux MIDI input records.
pub(crate) fn midi_input_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let backend = require_midi_input_handle_backend(
        binding,
        handle.0,
        "destack.device.midi.input.readBatch",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_read_batch(binding, handle, max_records, timeout_ns),
        MidiBackend::JackMidi => {
            jack::midi_input_read_batch(binding, handle, max_records, timeout_ns)
        }
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Poll one Linux MIDI input record without blocking.
pub(crate) fn midi_input_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    let backend =
        require_midi_input_handle_backend(binding, handle.0, "destack.device.midi.input.tryRead")?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_try_read(binding, handle),
        MidiBackend::JackMidi => jack::midi_input_try_read(binding, handle),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Poll one batch of Linux MIDI input records without blocking.
pub(crate) fn midi_input_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let backend = require_midi_input_handle_backend(
        binding,
        handle.0,
        "destack.device.midi.input.tryReadBatch",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_try_read_batch(binding, handle, max_records),
        MidiBackend::JackMidi => jack::midi_input_try_read_batch(binding, handle, max_records),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Create one virtual Linux MIDI input endpoint.
pub(crate) fn midi_input_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.input.virtual.create",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_input_virtual_create(binding, options),
        MidiBackend::JackMidi => jack::midi_input_virtual_create(binding, options),
        _ => unreachable!("linux MIDI backend resolution must return one host backend"),
    }
}

/// List Linux MIDI output endpoints.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.output.port.list",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_output_port_list(binding, options),
        MidiBackend::JackMidi => jack::midi_output_port_list(binding, options),
        _ => unreachable!("linux MIDI backend resolution must return one host backend"),
    }
}

/// Open one Linux MIDI output endpoint.
pub(crate) fn midi_output_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.output.port.open",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_output_port_open(binding, id, options),
        MidiBackend::JackMidi => jack::midi_output_port_open(binding, id, options),
        _ => unreachable!("linux MIDI backend resolution must return one host backend"),
    }
}

/// Describe one opened Linux MIDI output endpoint.
pub(crate) fn midi_output_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let backend = require_midi_output_handle_backend(
        binding,
        handle.0,
        "destack.device.midi.output.port.descriptor",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_output_port_descriptor(binding, handle),
        MidiBackend::JackMidi => jack::midi_output_port_descriptor(binding, handle),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Close one opened Linux MIDI output endpoint.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let backend = require_midi_output_handle_backend(
        binding,
        handle.0,
        "destack.device.midi.output.port.close",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_output_port_close(binding, handle),
        MidiBackend::JackMidi => jack::midi_output_port_close(binding, handle),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Write Linux MIDI output records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let backend =
        require_midi_output_handle_backend(binding, handle.0, "destack.device.midi.output.write")?;

    match backend {
        MidiBackend::Alsa => alsa::midi_output_write(binding, handle, records),
        MidiBackend::JackMidi => jack::midi_output_write(binding, handle, records),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Create one virtual Linux MIDI output endpoint.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.output.virtual.create",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_output_virtual_create(binding, options),
        MidiBackend::JackMidi => jack::midi_output_virtual_create(binding, options),
        _ => unreachable!("linux MIDI backend resolution must return one host backend"),
    }
}

/// Open one Linux MIDI event subscription.
pub(crate) fn midi_event_open(
    binding: &BindingCallContext,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<resource::MidiEventHandle> {
    let backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.event.open",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_event_open(binding, options),
        MidiBackend::JackMidi => jack::midi_event_open(binding, options),
        _ => unreachable!("linux MIDI backend resolution must return one host backend"),
    }
}

/// Close one opened Linux MIDI event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    let backend =
        require_midi_event_handle_backend(binding, handle.0, "destack.device.midi.event.close")?;

    match backend {
        MidiBackend::Alsa => alsa::midi_event_close(binding, handle),
        MidiBackend::JackMidi => jack::midi_event_close(binding, handle),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Wait for one Linux MIDI topology event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let backend =
        require_midi_event_handle_backend(binding, handle.0, "destack.device.midi.event.read")?;

    match backend {
        MidiBackend::Alsa => alsa::midi_event_read(binding, handle, timeout_ns),
        MidiBackend::JackMidi => jack::midi_event_read(binding, handle, timeout_ns),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Wait for one batch of Linux MIDI topology events.
pub(crate) fn midi_event_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let backend = require_midi_event_handle_backend(
        binding,
        handle.0,
        "destack.device.midi.event.readBatch",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_event_read_batch(binding, handle, max_events, timeout_ns),
        MidiBackend::JackMidi => {
            jack::midi_event_read_batch(binding, handle, max_events, timeout_ns)
        }
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Poll one Linux MIDI topology event without blocking.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let backend =
        require_midi_event_handle_backend(binding, handle.0, "destack.device.midi.event.tryRead")?;

    match backend {
        MidiBackend::Alsa => alsa::midi_event_try_read(binding, handle),
        MidiBackend::JackMidi => jack::midi_event_try_read(binding, handle),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}

/// Poll one batch of Linux MIDI topology events without blocking.
pub(crate) fn midi_event_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    max_events: u32,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let backend = require_midi_event_handle_backend(
        binding,
        handle.0,
        "destack.device.midi.event.tryReadBatch",
    )?;

    match backend {
        MidiBackend::Alsa => alsa::midi_event_try_read_batch(binding, handle, max_events),
        MidiBackend::JackMidi => jack::midi_event_try_read_batch(binding, handle, max_events),
        _ => unreachable!("linux MIDI handle routing must resolve one host backend"),
    }
}
