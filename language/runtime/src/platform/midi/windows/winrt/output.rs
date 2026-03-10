use std::sync::Arc;

use windows::Devices::Midi::MidiOutPort;
use windows::Storage::Streams::DataWriter;
use windows::core::Interface;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, validate_record_shape,
};
use crate::platform::midi::shared::remove_labeled_resource;
use crate::platform::midi::{
    MidiDataFormat, MidiOutputPortOpenOptions, MidiPortDirection, MidiPortListOptions,
    MidiProtocol, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::backend::resolve_backend;
use super::core::{WinRtOutputSession, insert_output_resource, missing_handle};
use super::descriptor::{
    filtered_descriptors, resolve_endpoint, validate_endpoint_transport_request,
};
use super::resource::output_resource;
use super::service::{ensure_current_thread_winrt_apartment, winrt_error, winrt_service};

/// Encode one outbound record into one WinRT byte buffer.
fn output_buffer(
    record: &MidiOutputRecordValue,
) -> RuntimeResult<windows::Storage::Streams::IBuffer> {
    let writer = DataWriter::new()
        .map_err(|error| winrt_error("destack.midi.output.write", "DataWriter::new", &error))?;

    writer.WriteBytes(&record.data).map_err(|error| {
        winrt_error(
            "destack.midi.output.write",
            "DataWriter::WriteBytes",
            &error,
        )
    })?;

    writer.DetachBuffer().map_err(|error| {
        winrt_error(
            "destack.midi.output.write",
            "DataWriter::DetachBuffer",
            &error,
        )
    })
}

/// List WinRT output ports.
pub(crate) fn midi_output_port_list(
    _binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.port.list",
    )?;
    let service = winrt_service("destack.midi.output.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Output,
        options.flags,
    ))
}

/// Open one WinRT output session.
pub(crate) fn midi_output_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let _runtime_state = binding.agent().platform_state.midi.runtime_state(binding);

    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.port.open",
    )?;
    let service = winrt_service("destack.midi.output.port.open")?;

    validate_record_shape(
        "destack.midi.output.port.open",
        options.data_format.unwrap_or(MidiDataFormat::Midi1Bytes),
        options.protocol,
    )?;

    // open the WinRT port on an initialized caller thread
    ensure_current_thread_winrt_apartment("destack.midi.output.port.open")?;

    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.midi.output.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let data_format = options
        .data_format
        .or(descriptor.default_data_format)
        .unwrap_or(MidiDataFormat::Midi1Bytes);
    let protocol = options.protocol.or(descriptor.default_protocol);

    validate_endpoint_transport_request(
        "destack.midi.output.port.open",
        &descriptor,
        data_format,
        protocol,
    )?;

    let port =
        MidiOutPort::FromIdAsync(&windows::core::HSTRING::from(endpoint.backend_id.as_str()))
            .map_err(|error| {
                winrt_error(
                    "destack.midi.output.port.open",
                    "MidiOutPort::FromIdAsync",
                    &error,
                )
            })?
            .get()
            .map_err(|error| {
                winrt_error(
                    "destack.midi.output.port.open",
                    "IAsyncOperation::get",
                    &error,
                )
            })?
            .cast::<MidiOutPort>()
            .map_err(|error| {
                winrt_error(
                    "destack.midi.output.port.open",
                    "IMidiOutPort::cast",
                    &error,
                )
            })?;

    let session = Arc::new(WinRtOutputSession {
        _service: service,
        descriptor,
        port,
    });

    Ok(insert_output_resource(binding, session))
}

/// Describe one opened WinRT output session.
pub(crate) fn midi_output_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = output_resource(binding, handle, "destack.midi.output.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened WinRT output session.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let _session = output_resource(binding, handle, "destack.midi.output.port.close")?;

    remove_labeled_resource(
        binding,
        handle.0,
        "destack.midi.output.port.close",
        "midi output port",
    )?;

    Ok(())
}

/// Write outbound WinRT MIDI records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let session = output_resource(binding, handle, "destack.midi.output.write")?;

    // write calls also touch WinRT objects on the current thread
    ensure_current_thread_winrt_apartment("destack.midi.output.write")?;

    for record in &records {
        validate_record_shape(
            "destack.midi.output.write",
            record.data_format,
            record.protocol,
        )?;

        if record.send_at_ns.is_some() {
            return Err(core_platform::invalid_argument(
                "records",
                "winrt midi output does not support scheduled send timestamps",
            ));
        }

        if record.data_format != MidiDataFormat::Midi1Bytes {
            return Err(core_platform::invalid_argument(
                "records",
                "winrt midi output only supports MIDI 1 byte-stream transport",
            ));
        }

        if record.protocol != Some(MidiProtocol::Midi1) && record.protocol.is_some() {
            return Err(core_platform::invalid_argument(
                "records",
                "winrt midi output only supports MIDI 1 protocol semantics",
            ));
        }

        let buffer = output_buffer(record)?;
        session.port.SendBuffer(&buffer).map_err(|error| {
            winrt_error(
                "destack.midi.output.write",
                "MidiOutPort::SendBuffer",
                &error,
            )
        })?;
    }

    let _ = binding;
    Ok(records.len() as u32)
}

/// Flush queued outbound WinRT MIDI records.
pub(crate) fn midi_output_flush(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let _session = output_resource(binding, handle, "destack.midi.output.flush")?;
    let _ = binding;

    Ok(())
}

/// Reject virtual output creation on WinRT.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    _options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let _runtime_state = binding.agent().platform_state.midi.runtime_state(binding);

    Err(core_platform::not_supported(
        "destack.midi.output.virtual.create",
    ))
}
