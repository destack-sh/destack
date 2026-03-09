use crate::platform::audio::{
    AudioBackendDisconnectedEvent, AudioBackendDisconnectedPayload, AudioBackendResetEvent,
    AudioBackendResetPayload, AudioDefaultCaptureChangedEvent, AudioDefaultCaptureChangedPayload,
    AudioDefaultLoopbackChangedEvent, AudioDefaultLoopbackChangedPayload,
    AudioDefaultPlaybackChangedEvent, AudioDefaultPlaybackChangedPayload, AudioDeviceAddedEvent,
    AudioDeviceAddedPayload, AudioDeviceFormatChangedEvent, AudioDeviceFormatChangedPayload,
    AudioDeviceRemovedEvent, AudioDeviceRemovedPayload, AudioDeviceReroutedEvent,
    AudioDeviceReroutedPayload, AudioEvent, AudioEventKind, AudioEventMetadata,
    AudioInterruptionBeganEvent, AudioInterruptionBeganPayload, AudioInterruptionEndedEvent,
    AudioInterruptionEndedPayload, AudioStreamDeviceChangedEvent, AudioStreamDeviceChangedPayload,
    AudioStreamStateChangedEvent, AudioStreamStateChangedPayload, AudioStreamXRunEvent,
    AudioStreamXRunPayload,
};
use crate::runtime::BindingCallContext;

use crate::platform::audio::core::AudioEventRecord;

/// Convert one stored event record into an ABI event payload.
pub(crate) fn abi_event(ctx: &BindingCallContext, event: AudioEventRecord) -> AudioEvent {
    let metadata = AudioEventMetadata {
        timestamp_ns: event.timestamp_ns,
        sequence: event.sequence,
        dropped_count: event.dropped_count,
        source: event.source,
        backend: event.backend,
        flags: event.flags,
    };
    let device_id = if event.device_id.is_empty() {
        None
    } else {
        Some(ctx.store_string(&event.device_id))
    };

    match event.kind {
        AudioEventKind::BackendDisconnected => {
            AudioEvent::AudioBackendDisconnectedEvent(AudioBackendDisconnectedEvent {
                kind: ctx.store_string("backendDisconnected"),
                metadata,
                payload: AudioBackendDisconnectedPayload {
                    stream: event.stream,
                },
            })
        }
        AudioEventKind::BackendReset => {
            AudioEvent::AudioBackendResetEvent(AudioBackendResetEvent {
                kind: ctx.store_string("backendReset"),
                metadata,
                payload: AudioBackendResetPayload {
                    stream: event.stream,
                },
            })
        }
        AudioEventKind::DefaultCaptureChanged => {
            AudioEvent::AudioDefaultCaptureChangedEvent(AudioDefaultCaptureChangedEvent {
                kind: ctx.store_string("defaultCaptureChanged"),
                metadata,
                payload: AudioDefaultCaptureChangedPayload { device_id },
            })
        }
        AudioEventKind::DefaultLoopbackChanged => {
            AudioEvent::AudioDefaultLoopbackChangedEvent(AudioDefaultLoopbackChangedEvent {
                kind: ctx.store_string("defaultLoopbackChanged"),
                metadata,
                payload: AudioDefaultLoopbackChangedPayload { device_id },
            })
        }
        AudioEventKind::DefaultPlaybackChanged => {
            AudioEvent::AudioDefaultPlaybackChangedEvent(AudioDefaultPlaybackChangedEvent {
                kind: ctx.store_string("defaultPlaybackChanged"),
                metadata,
                payload: AudioDefaultPlaybackChangedPayload { device_id },
            })
        }
        AudioEventKind::DeviceAdded => AudioEvent::AudioDeviceAddedEvent(AudioDeviceAddedEvent {
            kind: ctx.store_string("deviceAdded"),
            metadata,
            payload: AudioDeviceAddedPayload { device_id },
        }),
        AudioEventKind::DeviceFormatChanged => {
            AudioEvent::AudioDeviceFormatChangedEvent(AudioDeviceFormatChangedEvent {
                kind: ctx.store_string("deviceFormatChanged"),
                metadata,
                payload: AudioDeviceFormatChangedPayload { device_id },
            })
        }
        AudioEventKind::DeviceRemoved => {
            AudioEvent::AudioDeviceRemovedEvent(AudioDeviceRemovedEvent {
                kind: ctx.store_string("deviceRemoved"),
                metadata,
                payload: AudioDeviceRemovedPayload { device_id },
            })
        }
        AudioEventKind::DeviceRerouted => {
            AudioEvent::AudioDeviceReroutedEvent(AudioDeviceReroutedEvent {
                kind: ctx.store_string("deviceRerouted"),
                metadata,
                payload: AudioDeviceReroutedPayload { device_id },
            })
        }
        AudioEventKind::InterruptionBegan => {
            AudioEvent::AudioInterruptionBeganEvent(AudioInterruptionBeganEvent {
                kind: ctx.store_string("interruptionBegan"),
                metadata,
                payload: AudioInterruptionBeganPayload {
                    stream: event.stream,
                },
            })
        }
        AudioEventKind::InterruptionEnded => {
            AudioEvent::AudioInterruptionEndedEvent(AudioInterruptionEndedEvent {
                kind: ctx.store_string("interruptionEnded"),
                metadata,
                payload: AudioInterruptionEndedPayload {
                    stream: event.stream,
                },
            })
        }
        AudioEventKind::StreamDeviceChanged => {
            AudioEvent::AudioStreamDeviceChangedEvent(AudioStreamDeviceChangedEvent {
                kind: ctx.store_string("streamDeviceChanged"),
                metadata,
                payload: AudioStreamDeviceChangedPayload {
                    stream: event.stream,
                    status_flags: event.status_flags,
                    device_id,
                },
            })
        }
        AudioEventKind::StreamStateChanged => {
            AudioEvent::AudioStreamStateChangedEvent(AudioStreamStateChangedEvent {
                kind: ctx.store_string("streamStateChanged"),
                metadata,
                payload: AudioStreamStateChangedPayload {
                    stream: event.stream,
                    status_flags: event.status_flags,
                },
            })
        }
        AudioEventKind::StreamXRun => AudioEvent::AudioStreamXRunEvent(AudioStreamXRunEvent {
            kind: ctx.store_string("streamXRun"),
            metadata,
            payload: AudioStreamXRunPayload {
                stream: event.stream,
                status_flags: event.status_flags,
                xrun_count_delta: event.xrun_count_delta,
                device_id,
            },
        }),
    }
}
