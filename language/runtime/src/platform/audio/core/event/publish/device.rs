use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core::constants::{
    EVENT_SUBSCRIBE_DEFAULT_ROUTE, EVENT_SUBSCRIBE_DEVICE_HOTPLUG, EVENT_SUBSCRIBE_FORMAT_CHANGE,
    EVENT_SUBSCRIBE_REROUTE, host_monotonic_nanos,
};
use crate::platform::audio::core::event::queue::event_streams_snapshot;
use crate::platform::audio::core::event::snapshot::{MonitorSnapshot, monitor_snapshot};
use crate::platform::audio::core::model::{AudioEventStream, audio_device_monitor_baseline};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::platform::audio::core::monitor::active_audio_monitor_service;
use crate::platform::audio::core::runtime::{
    AudioRuntimeState, event_subscription_enabled, runtime_state, tracks_device_events,
};
use crate::platform::audio::{
    AudioBackend, AudioEventDeliveryMode, AudioEventKind, AudioEventSource,
};
use crate::runtime::BindingCallContext;

use super::core::publish_device_event;

/// Return active streams that accept one device event source for one backend.
fn active_device_publish_streams(
    runtime_state: &AudioRuntimeState,
    backend: AudioBackend,
    source: AudioEventSource,
) -> Vec<Arc<AudioEventStream>> {
    event_streams_snapshot(runtime_state)
        .into_iter()
        .filter(|stream| {
            stream.options.backend == backend
                && tracks_device_events(stream.options)
                && match source {
                    AudioEventSource::Native => {
                        stream.options.delivery_mode != AudioEventDeliveryMode::PollOnly
                    }
                    AudioEventSource::SyntheticPoll => {
                        stream.options.delivery_mode != AudioEventDeliveryMode::NativeOnly
                    }
                }
        })
        .collect()
}

/// Publish device deltas from one backend snapshot.
pub(crate) fn publish_device_events_from_snapshot(
    runtime_state: &AudioRuntimeState,
    backend: AudioBackend,
    snapshot: &MonitorSnapshot,
    now: u64,
    source: AudioEventSource,
) {
    let streams = active_device_publish_streams(runtime_state, backend, source);
    if streams.is_empty() {
        return;
    }

    let mut monitor_states = runtime_state
        .device_monitor_baselines
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some(previous_state) = monitor_states.get_mut(&backend) else {
        monitor_states.insert(
            backend,
            audio_device_monitor_baseline(
                snapshot.signatures.clone(),
                snapshot.default_playback.clone(),
                snapshot.default_capture.clone(),
                snapshot.default_loopback.clone(),
                now,
            ),
        );
        return;
    };

    for id in &snapshot.ids {
        // new devices
        if !previous_state.previous_signatures.contains_key(id)
            && streams.iter().any(|stream| {
                event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_DEVICE_HOTPLUG)
            })
        {
            publish_device_event(
                runtime_state,
                AudioEventKind::DeviceAdded,
                now,
                backend,
                id.clone(),
                source,
            );
            continue;
        }

        // changed devices
        if previous_state.previous_signatures.get(id) != snapshot.signatures.get(id) {
            if streams.iter().any(|stream| {
                event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_FORMAT_CHANGE)
            }) {
                publish_device_event(
                    runtime_state,
                    AudioEventKind::DeviceFormatChanged,
                    now,
                    backend,
                    id.clone(),
                    source,
                );
            }

            if streams
                .iter()
                .any(|stream| event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_REROUTE))
            {
                publish_device_event(
                    runtime_state,
                    AudioEventKind::DeviceRerouted,
                    now,
                    backend,
                    id.clone(),
                    source,
                );
            }
        }
    }

    // removed devices
    let previous_ids = previous_state
        .previous_signatures
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    for id in &previous_ids {
        if !snapshot.ids.contains(id)
            && streams.iter().any(|stream| {
                event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_DEVICE_HOTPLUG)
            })
        {
            publish_device_event(
                runtime_state,
                AudioEventKind::DeviceRemoved,
                now,
                backend,
                id.clone(),
                source,
            );
        }
    }

    // default route changes
    if previous_state.previous_default_playback != snapshot.default_playback
        && streams
            .iter()
            .any(|stream| event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_DEFAULT_ROUTE))
    {
        publish_device_event(
            runtime_state,
            AudioEventKind::DefaultPlaybackChanged,
            now,
            backend,
            snapshot.default_playback.clone().unwrap_or_default(),
            source,
        );
    }

    if previous_state.previous_default_capture != snapshot.default_capture
        && streams
            .iter()
            .any(|stream| event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_DEFAULT_ROUTE))
    {
        publish_device_event(
            runtime_state,
            AudioEventKind::DefaultCaptureChanged,
            now,
            backend,
            snapshot.default_capture.clone().unwrap_or_default(),
            source,
        );
    }

    if previous_state.previous_default_loopback != snapshot.default_loopback
        && streams
            .iter()
            .any(|stream| event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_DEFAULT_ROUTE))
    {
        publish_device_event(
            runtime_state,
            AudioEventKind::DefaultLoopbackChanged,
            now,
            backend,
            snapshot.default_loopback.clone().unwrap_or_default(),
            source,
        );
    }

    previous_state.previous_signatures = snapshot.signatures.clone();
    previous_state.previous_default_playback = snapshot.default_playback.clone();
    previous_state.previous_default_capture = snapshot.default_capture.clone();
    previous_state.previous_default_loopback = snapshot.default_loopback.clone();
    previous_state.last_refresh_ns = now;
}

/// Publish one backend-native device snapshot diff to the live monitor service.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub(crate) fn publish_device_snapshot_native_if_service_live(backend: AudioBackend) {
    // ignore callbacks that race monitor service startup or teardown
    let Some(service) = active_audio_monitor_service() else {
        return;
    };

    service.publish_native_snapshot(backend);
}

/// Force one immediate refresh pass for device subscriptions on one backend.
pub(crate) fn refresh_device_subscriptions_for_rescan(
    ctx: &BindingCallContext,
    backend: AudioBackend,
) -> RuntimeResult<()> {
    let runtime_state = runtime_state(ctx);
    let now = host_monotonic_nanos();
    let snapshot = monitor_snapshot(backend)?;
    publish_device_events_from_snapshot(
        &runtime_state,
        backend,
        &snapshot,
        now,
        AudioEventSource::SyntheticPoll,
    );

    Ok(())
}
