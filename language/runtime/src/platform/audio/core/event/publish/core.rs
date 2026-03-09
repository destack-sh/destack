use crate::platform::audio::{
    AudioBackend, AudioEventKind, AudioEventSource, AudioStreamStatusFlags,
};
use crate::platform::resource::AudioStreamHandle;

use super::super::queue::publish_event_record;
use crate::platform::audio::core::model::AudioEventRecord;
use crate::platform::audio::core::runtime::AudioRuntimeState;

/// Publish one device-level event record.
pub(crate) fn publish_device_event(
    runtime_state: &AudioRuntimeState,
    kind: AudioEventKind,
    timestamp_ns: u64,
    backend: AudioBackend,
    device_id: String,
    source: AudioEventSource,
) {
    publish_event_record(
        runtime_state,
        AudioEventRecord {
            kind,
            timestamp_ns,
            sequence: 0,
            dropped_count: 0,
            source,
            backend,
            device_id,
            flags: 0,
            status_flags: AudioStreamStatusFlags(0),
            xrun_count_delta: 0,
            stream: None,
        },
    );
}

/// Publish one stream-level event record.
pub(crate) fn publish_stream_event(
    runtime_state: &AudioRuntimeState,
    kind: AudioEventKind,
    timestamp_ns: u64,
    backend: AudioBackend,
    device_id: String,
    status_flags: AudioStreamStatusFlags,
    xrun_count_delta: u64,
    stream: AudioStreamHandle,
    source: AudioEventSource,
) {
    publish_event_record(
        runtime_state,
        AudioEventRecord {
            kind,
            timestamp_ns,
            sequence: 0,
            dropped_count: 0,
            source,
            backend,
            device_id,
            flags: 0,
            status_flags,
            xrun_count_delta,
            stream: Some(stream),
        },
    );
}
