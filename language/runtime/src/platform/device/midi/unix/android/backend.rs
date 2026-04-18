use crate::diagnostic::RuntimeResult;
use crate::platform::core::{BackendSupport, backend_support_from_check};
use crate::platform::device::midi::core::{
    MidiBackendDescriptorValue, MidiBackendMetadata, midi_backend_descriptors,
    midi_backend_metadata, midi_selector_support, resolve_midi_backend_selection,
};
use crate::platform::device::{MidiBackend, MidiBackendSelectionPolicy};
use crate::runtime::BindingCallContext;

use super::core::AndroidBackendDescription;

/// Host backend selectors considered by auto selection on Android.
const PREFERRED_HOST_BACKENDS: [MidiBackend; 1] = [MidiBackend::AndroidMidi];

/// Return host support for one Android selector row.
fn backend_support(binding: &BindingCallContext, backend: MidiBackend) -> BackendSupport {
    midi_selector_support(&PREFERRED_HOST_BACKENDS, backend, |backend| match backend {
        MidiBackend::AndroidMidi => Some(android_backend_support(binding)),
        _ => None,
    })
}

/// Probe Android host reachability for the Android MIDI backend.
fn android_backend_support(binding: &BindingCallContext) -> BackendSupport {
    let description = binding
        .worker()
        .platform_state
        .device
        .describe_android_backend(binding, "destack.device.midi.backend.support");

    backend_support_from_check("android-midi", description)
}

/// Return one process-wide Android backend description when available.
fn backend_description(binding: &BindingCallContext) -> Option<AndroidBackendDescription> {
    binding
        .worker()
        .platform_state
        .device
        .describe_android_backend(binding, "destack.device.midi.backend.list")
        .ok()
}

/// Resolve one effective Android backend.
pub(super) fn resolve_backend(
    binding: &BindingCallContext,
    backend: MidiBackend,
    policy: MidiBackendSelectionPolicy,
    operation: &'static str,
) -> RuntimeResult<MidiBackend> {
    resolve_midi_backend_selection(
        &PREFERRED_HOST_BACKENDS,
        backend,
        policy,
        operation,
        |backend| backend_support(binding, backend),
    )
}

/// Return selector-row metadata for one Android selector row.
fn backend_metadata(
    description: Option<AndroidBackendDescription>,
    backend: MidiBackend,
    support: BackendSupport,
) -> MidiBackendMetadata {
    if backend != MidiBackend::AndroidMidi || !support.is_available() {
        return MidiBackendMetadata::default();
    }

    match description {
        Some(description) => midi_backend_metadata(
            description.capability_flags,
            description.supported_data_formats,
            description.supported_protocols,
        ),
        None => MidiBackendMetadata::default(),
    }
}

/// Build backend descriptors for Android MIDI hosts.
pub(crate) fn midi_backend_list(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<MidiBackendDescriptorValue>> {
    let description = backend_description(binding);

    Ok(midi_backend_descriptors(
        &PREFERRED_HOST_BACKENDS,
        |backend| backend_support(binding, backend),
        |backend, support| backend_metadata(description, backend, support),
    ))
}
