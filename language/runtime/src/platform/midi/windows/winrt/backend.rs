use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform, BackendSupport, aggregate_backend_support};
use crate::platform::midi::core::MidiBackendDescriptorValue;
use crate::platform::midi::selector::{MIDI_SELECTOR_ROWS, midi_backend_name};
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1,
    MidiBackend, MidiBackendCapabilityFlags, MidiBackendSelectionPolicy, MidiDataFormatFlags,
    MidiProtocolFlags,
};
use crate::runtime::BindingCallContext;

/// Host backend selectors considered by auto selection on Windows.
const PREFERRED_HOST_BACKENDS: [MidiBackend; 2] = [MidiBackend::WinRT, MidiBackend::WinMM];

/// Return host support for one backend selector on Windows.
pub(super) fn backend_support(backend: MidiBackend) -> BackendSupport {
    match backend {
        MidiBackend::Auto => {
            aggregate_backend_support(PREFERRED_HOST_BACKENDS.map(backend_support))
        }
        MidiBackend::WinRT => BackendSupport::Available,
        MidiBackend::WinMM => BackendSupport::DisabledByBuild,
        MidiBackend::Null => BackendSupport::Available,
        _ => BackendSupport::UnsupportedTarget,
    }
}

/// Resolve the first usable Windows host backend for auto selection.
fn resolve_auto_backend() -> Option<MidiBackend> {
    PREFERRED_HOST_BACKENDS
        .into_iter()
        .find(|backend| backend_support(*backend).is_available())
}

/// Resolve one effective host backend.
pub(super) fn resolve_backend(
    backend: MidiBackend,
    policy: MidiBackendSelectionPolicy,
    operation: &'static str,
) -> RuntimeResult<MidiBackend> {
    // auto selection
    if backend == MidiBackend::Auto {
        return resolve_auto_backend().ok_or_else(|| {
            core_platform::backend_support_error(
                operation,
                midi_backend_name(MidiBackend::Auto),
                backend_support(MidiBackend::Auto),
            )
        });
    }

    // direct host backend selection
    if backend == MidiBackend::WinRT {
        return Ok(MidiBackend::WinRT);
    }

    // keep the null backend out of the host implementation surface
    if backend == MidiBackend::Null {
        return Err(core_platform::not_supported(format!(
            "{operation}: backend {} does not expose one host MIDI implementation",
            midi_backend_name(backend),
        )));
    }

    // fallback to the preferred host backend when requested
    if policy == MidiBackendSelectionPolicy::AllowFallback
        && let Some(backend) = resolve_auto_backend()
    {
        return Ok(backend);
    }

    Err(core_platform::backend_support_error(
        operation,
        midi_backend_name(backend),
        backend_support(backend),
    ))
}

/// Return backend capability flags for one selector row.
fn backend_capability_flags(
    backend: MidiBackend,
    support: BackendSupport,
) -> MidiBackendCapabilityFlags {
    if !support.is_available() || backend == MidiBackend::Null {
        return MidiBackendCapabilityFlags(0);
    }

    let mut flags = 0u64;
    flags |= MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0;
    flags |= MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0;
    flags |= MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0;

    MidiBackendCapabilityFlags(flags)
}

/// Return advertised transport data formats for one selector row.
fn backend_supported_data_formats(
    backend: MidiBackend,
    support: BackendSupport,
) -> MidiDataFormatFlags {
    if !support.is_available() || backend == MidiBackend::Null {
        return MidiDataFormatFlags(0);
    }

    MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0)
}

/// Return advertised protocol flags for one selector row.
fn backend_supported_protocols(backend: MidiBackend, support: BackendSupport) -> MidiProtocolFlags {
    if !support.is_available() || backend == MidiBackend::Null {
        return MidiProtocolFlags(0);
    }

    MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0)
}

/// Return one auto-selection priority for one selector row.
fn backend_priority(backend: MidiBackend) -> u16 {
    match backend {
        MidiBackend::Auto => u16::MAX,
        _ => PREFERRED_HOST_BACKENDS
            .iter()
            .position(|candidate| *candidate == backend)
            .map(|index| u16::MAX - 1 - index as u16)
            .unwrap_or(0),
    }
}

/// Build backend descriptors for the WinRT host.
pub(crate) fn midi_backend_list(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<MidiBackendDescriptorValue>> {
    binding
        .agent()
        .platform_state
        .midi
        .ensure_winrt_service("destack.midi.backend.list")?;

    let descriptors = MIDI_SELECTOR_ROWS
        .into_iter()
        .map(|backend| {
            let support = backend_support(backend);

            MidiBackendDescriptorValue {
                backend,
                name: midi_backend_name(backend),
                support,
                priority: backend_priority(backend),
                capability_flags: backend_capability_flags(backend, support),
                supported_data_formats: backend_supported_data_formats(backend, support),
                supported_protocols: backend_supported_protocols(backend, support),
            }
        })
        .collect();

    Ok(descriptors)
}
