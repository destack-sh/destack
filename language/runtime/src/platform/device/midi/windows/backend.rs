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

use super::{midi, winmm, winrt};

/// Host backend selectors considered by auto selection on Windows.
const PREFERRED_HOST_BACKENDS: [MidiBackend; 3] = [
    MidiBackend::WindowsMidi,
    MidiBackend::WinRT,
    MidiBackend::WinMM,
];

/// Return host support for one backend selector on Windows.
fn backend_support(backend: MidiBackend) -> BackendSupport {
    midi_selector_support(&PREFERRED_HOST_BACKENDS, backend, |backend| match backend {
        MidiBackend::WindowsMidi => Some(midi::backend_support()),
        MidiBackend::WinRT => Some(winrt::backend_support()),
        MidiBackend::WinMM => Some(winmm::backend_support()),
        _ => None,
    })
}

/// Resolve one effective Windows host backend.
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

/// Build backend descriptors for Windows MIDI hosts.
pub(crate) fn midi_backend_list(
    _binding: &BindingCallContext,
) -> RuntimeResult<Vec<MidiBackendDescriptorValue>> {
    Ok(midi_backend_descriptors(
        &PREFERRED_HOST_BACKENDS,
        backend_support,
        |backend, support| match backend {
            MidiBackend::WindowsMidi => midi::backend_metadata(support),
            MidiBackend::WinRT => winrt::backend_metadata(support),
            MidiBackend::WinMM => winmm::backend_metadata(support),
            _ => Default::default(),
        },
    ))
}

/// List Windows MIDI input endpoints.
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
        MidiBackend::WindowsMidi => midi::midi_input_port_list(binding, options),
        MidiBackend::WinRT => winrt::midi_input_port_list(binding, options),
        MidiBackend::WinMM => winmm::midi_input_port_list(binding, options),
        _ => unreachable!("windows MIDI backend resolution must return one host backend"),
    }
}

/// Open one Windows MIDI input endpoint.
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
        MidiBackend::WindowsMidi => midi::midi_input_port_open(binding, id, options),
        MidiBackend::WinRT => winrt::midi_input_port_open(binding, id, options),
        MidiBackend::WinMM => winmm::midi_input_port_open(binding, id, options),
        _ => unreachable!("windows MIDI backend resolution must return one host backend"),
    }
}

/// Describe one opened Windows MIDI input endpoint.
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
        MidiBackend::WindowsMidi => midi::midi_input_port_descriptor(binding, handle),
        MidiBackend::WinRT => winrt::midi_input_port_descriptor(binding, handle),
        MidiBackend::WinMM => winmm::midi_input_port_descriptor(binding, handle),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Close one opened Windows MIDI input endpoint.
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
        MidiBackend::WindowsMidi => midi::midi_input_port_close(binding, handle),
        MidiBackend::WinRT => winrt::midi_input_port_close(binding, handle),
        MidiBackend::WinMM => winmm::midi_input_port_close(binding, handle),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Wait for one Windows MIDI input record.
pub(crate) fn midi_input_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiInputRecordValue> {
    let backend =
        require_midi_input_handle_backend(binding, handle.0, "destack.device.midi.input.read")?;

    match backend {
        MidiBackend::WindowsMidi => midi::midi_input_read(binding, handle, timeout_ns),
        MidiBackend::WinRT => winrt::midi_input_read(binding, handle, timeout_ns),
        MidiBackend::WinMM => winmm::midi_input_read(binding, handle, timeout_ns),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Wait for one batch of Windows MIDI input records.
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
        MidiBackend::WindowsMidi => {
            midi::midi_input_read_batch(binding, handle, max_records, timeout_ns)
        }
        MidiBackend::WinRT => {
            winrt::midi_input_read_batch(binding, handle, max_records, timeout_ns)
        }
        MidiBackend::WinMM => {
            winmm::midi_input_read_batch(binding, handle, max_records, timeout_ns)
        }
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Poll one Windows MIDI input record.
pub(crate) fn midi_input_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    let backend =
        require_midi_input_handle_backend(binding, handle.0, "destack.device.midi.input.tryRead")?;

    match backend {
        MidiBackend::WindowsMidi => midi::midi_input_try_read(binding, handle),
        MidiBackend::WinRT => winrt::midi_input_try_read(binding, handle),
        MidiBackend::WinMM => winmm::midi_input_try_read(binding, handle),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Poll one batch of Windows MIDI input records.
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
        MidiBackend::WindowsMidi => midi::midi_input_try_read_batch(binding, handle, max_records),
        MidiBackend::WinRT => winrt::midi_input_try_read_batch(binding, handle, max_records),
        MidiBackend::WinMM => winmm::midi_input_try_read_batch(binding, handle, max_records),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Create one Windows virtual input endpoint.
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
        MidiBackend::WindowsMidi => midi::midi_input_virtual_create(binding, options),
        MidiBackend::WinRT => winrt::midi_input_virtual_create(binding, options),
        MidiBackend::WinMM => winmm::midi_input_virtual_create(binding, options),
        _ => unreachable!("windows MIDI backend resolution must return one host backend"),
    }
}

/// List Windows MIDI output endpoints.
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
        MidiBackend::WindowsMidi => midi::midi_output_port_list(binding, options),
        MidiBackend::WinRT => winrt::midi_output_port_list(binding, options),
        MidiBackend::WinMM => winmm::midi_output_port_list(binding, options),
        _ => unreachable!("windows MIDI backend resolution must return one host backend"),
    }
}

/// Open one Windows MIDI output endpoint.
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
        MidiBackend::WindowsMidi => midi::midi_output_port_open(binding, id, options),
        MidiBackend::WinRT => winrt::midi_output_port_open(binding, id, options),
        MidiBackend::WinMM => winmm::midi_output_port_open(binding, id, options),
        _ => unreachable!("windows MIDI backend resolution must return one host backend"),
    }
}

/// Describe one opened Windows MIDI output endpoint.
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
        MidiBackend::WindowsMidi => midi::midi_output_port_descriptor(binding, handle),
        MidiBackend::WinRT => winrt::midi_output_port_descriptor(binding, handle),
        MidiBackend::WinMM => winmm::midi_output_port_descriptor(binding, handle),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Close one opened Windows MIDI output endpoint.
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
        MidiBackend::WindowsMidi => midi::midi_output_port_close(binding, handle),
        MidiBackend::WinRT => winrt::midi_output_port_close(binding, handle),
        MidiBackend::WinMM => winmm::midi_output_port_close(binding, handle),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Write one Windows MIDI output batch.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let backend =
        require_midi_output_handle_backend(binding, handle.0, "destack.device.midi.output.write")?;

    match backend {
        MidiBackend::WindowsMidi => midi::midi_output_write(binding, handle, records),
        MidiBackend::WinRT => winrt::midi_output_write(binding, handle, records),
        MidiBackend::WinMM => winmm::midi_output_write(binding, handle, records),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Create one Windows virtual output endpoint.
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
        MidiBackend::WindowsMidi => midi::midi_output_virtual_create(binding, options),
        MidiBackend::WinRT => winrt::midi_output_virtual_create(binding, options),
        MidiBackend::WinMM => winmm::midi_output_virtual_create(binding, options),
        _ => unreachable!("windows MIDI backend resolution must return one host backend"),
    }
}

/// Open one Windows MIDI event subscription.
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
        MidiBackend::WindowsMidi => midi::midi_event_open(binding, options),
        MidiBackend::WinRT => winrt::midi_event_open(binding, options),
        MidiBackend::WinMM => winmm::midi_event_open(binding, options),
        _ => unreachable!("windows MIDI backend resolution must return one host backend"),
    }
}

/// Close one opened Windows MIDI event subscription.
pub(crate) fn midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    let backend =
        require_midi_event_handle_backend(binding, handle.0, "destack.device.midi.event.close")?;

    match backend {
        MidiBackend::WindowsMidi => midi::midi_event_close(binding, handle),
        MidiBackend::WinRT => winrt::midi_event_close(binding, handle),
        MidiBackend::WinMM => winmm::midi_event_close(binding, handle),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Wait for one Windows MIDI topology event.
pub(crate) fn midi_event_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiEventValue> {
    let backend =
        require_midi_event_handle_backend(binding, handle.0, "destack.device.midi.event.read")?;

    match backend {
        MidiBackend::WindowsMidi => midi::midi_event_read(binding, handle, timeout_ns),
        MidiBackend::WinRT => winrt::midi_event_read(binding, handle, timeout_ns),
        MidiBackend::WinMM => winmm::midi_event_read(binding, handle, timeout_ns),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Wait for one batch of Windows MIDI topology events.
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
        MidiBackend::WindowsMidi => {
            midi::midi_event_read_batch(binding, handle, max_events, timeout_ns)
        }
        MidiBackend::WinRT => winrt::midi_event_read_batch(binding, handle, max_events, timeout_ns),
        MidiBackend::WinMM => winmm::midi_event_read_batch(binding, handle, max_events, timeout_ns),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Poll one Windows MIDI topology event.
pub(crate) fn midi_event_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventValue> {
    let backend =
        require_midi_event_handle_backend(binding, handle.0, "destack.device.midi.event.tryRead")?;

    match backend {
        MidiBackend::WindowsMidi => midi::midi_event_try_read(binding, handle),
        MidiBackend::WinRT => winrt::midi_event_try_read(binding, handle),
        MidiBackend::WinMM => winmm::midi_event_try_read(binding, handle),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}

/// Poll one batch of Windows MIDI topology events.
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
        MidiBackend::WindowsMidi => midi::midi_event_try_read_batch(binding, handle, max_events),
        MidiBackend::WinRT => winrt::midi_event_try_read_batch(binding, handle, max_events),
        MidiBackend::WinMM => winmm::midi_event_try_read_batch(binding, handle, max_events),
        _ => unreachable!("windows MIDI handle routing must resolve one host backend"),
    }
}
