use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform, BackendSupport, aggregate_backend_support};
use crate::platform::device::midi::selector::{MIDI_SELECTOR_ROWS, midi_backend_name};
use crate::platform::device::{
    MidiBackend, MidiBackendCapabilityFlags, MidiBackendSelectionPolicy, MidiDataFormatFlags,
    MidiProtocolFlags,
};

use super::value::MidiBackendDescriptorValue;

/// One advertised selector-row metadata payload.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MidiBackendMetadata {
    /// Advertised backend capability flags.
    pub(crate) capability_flags: MidiBackendCapabilityFlags,
    /// Advertised transport data formats.
    pub(crate) supported_data_formats: MidiDataFormatFlags,
    /// Advertised protocol flags.
    pub(crate) supported_protocols: MidiProtocolFlags,
}

impl Default for MidiBackendMetadata {
    /// Return zeroed selector-row metadata.
    fn default() -> Self {
        midi_backend_metadata(
            MidiBackendCapabilityFlags(0),
            MidiDataFormatFlags(0),
            MidiProtocolFlags(0),
        )
    }
}

/// Build one metadata payload from explicit selector-row fields.
pub(crate) const fn midi_backend_metadata(
    capability_flags: MidiBackendCapabilityFlags,
    supported_data_formats: MidiDataFormatFlags,
    supported_protocols: MidiProtocolFlags,
) -> MidiBackendMetadata {
    MidiBackendMetadata {
        capability_flags,
        supported_data_formats,
        supported_protocols,
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows,
    test
))]
/// Return zeroed metadata for one unavailable selector row.
pub(crate) fn available_midi_backend_metadata(
    support: BackendSupport,
    metadata: MidiBackendMetadata,
) -> MidiBackendMetadata {
    if support.is_available() {
        return metadata;
    }

    MidiBackendMetadata::default()
}

/// Return selector-row support for one host lane with one preferred backend order.
pub(crate) fn midi_selector_support(
    preferred_backends: &[MidiBackend],
    backend: MidiBackend,
    mut concrete_support: impl FnMut(MidiBackend) -> Option<BackendSupport>,
) -> BackendSupport {
    match backend {
        MidiBackend::Auto => {
            aggregate_backend_support(preferred_backends.iter().copied().map(|backend| {
                concrete_support(backend).unwrap_or(BackendSupport::UnsupportedTarget)
            }))
        }
        MidiBackend::Null => BackendSupport::Available,
        _ => concrete_support(backend).unwrap_or(BackendSupport::UnsupportedTarget),
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows,
    test
))]
/// Return one metadata payload for one specific concrete selector row.
pub(crate) fn single_midi_backend_metadata(
    backend: MidiBackend,
    supported_backend: MidiBackend,
    support: BackendSupport,
    metadata: MidiBackendMetadata,
) -> MidiBackendMetadata {
    if backend != supported_backend {
        return MidiBackendMetadata::default();
    }

    available_midi_backend_metadata(support, metadata)
}

/// Resolve the first usable host backend in one selector lane.
pub(crate) fn resolve_auto_midi_backend(
    preferred_backends: &[MidiBackend],
    mut backend_support: impl FnMut(MidiBackend) -> BackendSupport,
) -> Option<MidiBackend> {
    preferred_backends
        .iter()
        .copied()
        .find(|backend| backend_support(*backend).is_available())
}

/// Resolve one effective host backend from one selector lane.
pub(crate) fn resolve_midi_backend_selection(
    preferred_backends: &[MidiBackend],
    backend: MidiBackend,
    policy: MidiBackendSelectionPolicy,
    operation: &'static str,
    mut backend_support: impl FnMut(MidiBackend) -> BackendSupport,
) -> RuntimeResult<MidiBackend> {
    // auto selection
    if backend == MidiBackend::Auto {
        return resolve_auto_midi_backend(preferred_backends, &mut backend_support).ok_or_else(
            || {
                core_platform::backend_support_error(
                    operation,
                    midi_backend_name(MidiBackend::Auto),
                    backend_support(MidiBackend::Auto),
                )
            },
        );
    }

    // keep the null backend out of the host implementation surface
    if backend == MidiBackend::Null {
        return Err(core_platform::not_supported(format!(
            "{operation}: backend {} does not expose one host MIDI implementation",
            midi_backend_name(backend),
        )));
    }

    // direct host backend selection
    let support = backend_support(backend);
    if support.is_available() {
        return Ok(backend);
    }

    // fallback to the preferred host backend when requested
    if policy == MidiBackendSelectionPolicy::AllowFallback
        && let Some(backend) = resolve_auto_midi_backend(preferred_backends, &mut backend_support)
    {
        return Ok(backend);
    }

    Err(core_platform::backend_support_error(
        operation,
        midi_backend_name(backend),
        support,
    ))
}

/// Return one auto-selection priority for one selector row.
pub(crate) fn midi_backend_priority(
    preferred_backends: &[MidiBackend],
    backend: MidiBackend,
) -> u16 {
    match backend {
        MidiBackend::Auto => u16::MAX,
        _ => preferred_backends
            .iter()
            .position(|candidate| *candidate == backend)
            .map(|index| u16::MAX - 1 - index as u16)
            .unwrap_or(0),
    }
}

/// Build backend descriptors for one selector lane.
pub(crate) fn midi_backend_descriptors(
    preferred_backends: &[MidiBackend],
    mut backend_support: impl FnMut(MidiBackend) -> BackendSupport,
    mut backend_metadata: impl FnMut(MidiBackend, BackendSupport) -> MidiBackendMetadata,
) -> Vec<MidiBackendDescriptorValue> {
    MIDI_SELECTOR_ROWS
        .into_iter()
        .map(|backend| {
            let support = backend_support(backend);
            let metadata = backend_metadata(backend, support);

            MidiBackendDescriptorValue {
                backend,
                name: midi_backend_name(backend),
                support,
                priority: midi_backend_priority(preferred_backends, backend),
                capability_flags: metadata.capability_flags,
                supported_data_formats: metadata.supported_data_formats,
                supported_protocols: metadata.supported_protocols,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        MidiBackendMetadata, available_midi_backend_metadata, midi_backend_descriptors,
        midi_backend_metadata, midi_selector_support, resolve_midi_backend_selection,
        single_midi_backend_metadata,
    };
    use crate::platform::core::BackendSupport;
    use crate::platform::device::{
        MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES,
        MIDI_PROTOCOL_FLAG_MIDI1, MidiBackend, MidiBackendCapabilityFlags,
        MidiBackendSelectionPolicy, MidiDataFormatFlags, MidiProtocolFlags,
    };
    use crate::platform::diagnostic::PlatformErrorCode;

    /// Resolve auto selection to the first available preferred backend.
    #[test]
    fn test_resolve_midi_backend_selection_prefers_the_first_available_backend_for_auto() {
        let preferred_backends = [MidiBackend::WindowsMidi, MidiBackend::WinRT];

        let backend = resolve_midi_backend_selection(
            &preferred_backends,
            MidiBackend::Auto,
            MidiBackendSelectionPolicy::Strict,
            "destack.device.midi.backend.test",
            |backend| match backend {
                MidiBackend::Auto => BackendSupport::Available,
                MidiBackend::WindowsMidi => BackendSupport::HostUnavailable,
                MidiBackend::WinRT => BackendSupport::Available,
                _ => BackendSupport::UnsupportedTarget,
            },
        )
        .expect("auto selection should resolve the first available backend");

        assert_eq!(backend, MidiBackend::WinRT);
    }

    /// Fall back to one available preferred backend when the requested selector is unavailable.
    #[test]
    fn test_resolve_midi_backend_selection_falls_back_when_policy_allows() {
        let preferred_backends = [MidiBackend::WindowsMidi, MidiBackend::WinRT];

        let backend = resolve_midi_backend_selection(
            &preferred_backends,
            MidiBackend::JackMidi,
            MidiBackendSelectionPolicy::AllowFallback,
            "destack.device.midi.backend.test",
            |backend| match backend {
                MidiBackend::Auto => BackendSupport::Available,
                MidiBackend::WindowsMidi => BackendSupport::Available,
                MidiBackend::WinRT => BackendSupport::HostUnavailable,
                MidiBackend::JackMidi => BackendSupport::UnsupportedTarget,
                _ => BackendSupport::UnsupportedTarget,
            },
        )
        .expect("fallback selection should return one available preferred backend");

        assert_eq!(backend, MidiBackend::WindowsMidi);
    }

    /// Reject the null backend selector as one concrete host implementation.
    #[test]
    fn test_resolve_midi_backend_selection_rejects_the_null_backend() {
        let error = resolve_midi_backend_selection(
            &[MidiBackend::WindowsMidi],
            MidiBackend::Null,
            MidiBackendSelectionPolicy::Strict,
            "destack.device.midi.backend.test",
            |_| BackendSupport::Available,
        )
        .expect_err("null backend selection should fail");

        assert_eq!(
            error.platform_error().map(|error| error.code),
            Some(PlatformErrorCode::NotSupported),
        );
    }

    /// Zero selector-row metadata when the backend is unavailable.
    #[test]
    fn test_available_midi_backend_metadata_zeroes_unavailable_rows() {
        let metadata = midi_backend_metadata(
            MidiBackendCapabilityFlags(MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0),
            MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
            MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
        );

        let available = available_midi_backend_metadata(BackendSupport::Available, metadata);
        let unavailable =
            available_midi_backend_metadata(BackendSupport::HostUnavailable, metadata);

        assert_eq!(available.capability_flags.0, metadata.capability_flags.0);
        assert_eq!(unavailable.capability_flags.0, 0);
        assert_eq!(unavailable.supported_data_formats.0, 0);
        assert_eq!(unavailable.supported_protocols.0, 0);
    }

    /// Aggregate selector support across one preferred backend lane.
    #[test]
    fn test_midi_selector_support_aggregates_auto_and_null_rows() {
        let preferred_backends = [MidiBackend::WindowsMidi, MidiBackend::WinRT];

        let auto_support = midi_selector_support(
            &preferred_backends,
            MidiBackend::Auto,
            |backend| match backend {
                MidiBackend::WindowsMidi => Some(BackendSupport::HostUnavailable),
                MidiBackend::WinRT => Some(BackendSupport::DisabledByBuild),
                _ => None,
            },
        );
        let null_support =
            midi_selector_support(&preferred_backends, MidiBackend::Null, |_backend| None);
        let unsupported_support =
            midi_selector_support(&preferred_backends, MidiBackend::JackMidi, |_backend| None);

        assert_eq!(auto_support, BackendSupport::HostUnavailable);
        assert_eq!(null_support, BackendSupport::Available);
        assert_eq!(unsupported_support, BackendSupport::UnsupportedTarget);
    }

    /// Keep single-backend metadata helpers zeroed for other selector rows.
    #[test]
    fn test_single_midi_backend_metadata_only_applies_to_the_matching_selector() {
        let metadata = midi_backend_metadata(
            MidiBackendCapabilityFlags(MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0),
            MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
            MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
        );

        let matching = single_midi_backend_metadata(
            MidiBackend::WinRT,
            MidiBackend::WinRT,
            BackendSupport::Available,
            metadata,
        );
        let non_matching = single_midi_backend_metadata(
            MidiBackend::WindowsMidi,
            MidiBackend::WinRT,
            BackendSupport::Available,
            metadata,
        );

        assert_eq!(matching.capability_flags.0, metadata.capability_flags.0);
        assert_eq!(non_matching.capability_flags.0, 0);
        assert_eq!(non_matching.supported_data_formats.0, 0);
        assert_eq!(non_matching.supported_protocols.0, 0);
    }

    /// Keep descriptor priorities aligned with the preferred selector order.
    #[test]
    fn test_midi_backend_descriptors_follow_preferred_priority_order() {
        let preferred_backends = [MidiBackend::WindowsMidi, MidiBackend::WinRT];
        let metadata = MidiBackendMetadata::default();
        let descriptors = midi_backend_descriptors(
            &preferred_backends,
            |backend| match backend {
                MidiBackend::Auto | MidiBackend::WindowsMidi | MidiBackend::WinRT => {
                    BackendSupport::Available
                }
                MidiBackend::Null => BackendSupport::Available,
                _ => BackendSupport::UnsupportedTarget,
            },
            |_backend, _support| metadata,
        );

        let auto = descriptors
            .iter()
            .find(|descriptor| descriptor.backend == MidiBackend::Auto)
            .expect("auto descriptor should exist");
        let windows_midi = descriptors
            .iter()
            .find(|descriptor| descriptor.backend == MidiBackend::WindowsMidi)
            .expect("windows-midi descriptor should exist");
        let winrt = descriptors
            .iter()
            .find(|descriptor| descriptor.backend == MidiBackend::WinRT)
            .expect("winrt descriptor should exist");

        assert!(auto.priority > windows_midi.priority);
        assert!(windows_midi.priority > winrt.priority);
    }
}
