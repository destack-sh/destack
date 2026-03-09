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
use crate::platform::audio::core::monitor::AudioMonitorServiceRegistry;
use crate::platform::audio::core::runtime::{
    AudioRuntimeState, event_subscription_enabled, runtime_state, tracks_device_events,
};
use crate::platform::audio::{
    AudioBackend, AudioEventDeliveryMode, AudioEventKind, AudioEventSource,
};
use crate::runtime::BindingCallContext;

use super::core::publish_device_event;

/// Return active streams that request native device events for one backend.
pub(crate) fn active_native_device_publish_streams(
    runtime_state: &AudioRuntimeState,
    backend: AudioBackend,
) -> Vec<Arc<AudioEventStream>> {
    event_streams_snapshot(runtime_state)
        .into_iter()
        .filter(|stream| {
            stream.options.backend == backend
                && stream.options.delivery_mode != AudioEventDeliveryMode::PollOnly
                && tracks_device_events(stream.options)
        })
        .collect()
}

/// Return whether one backend device-monitor refresh is due.
fn device_monitor_refresh_due(
    runtime_state: &AudioRuntimeState,
    backend: AudioBackend,
    poll_interval_ns: u64,
    now: u64,
) -> bool {
    let monitor_states = runtime_state
        .device_monitor_baselines
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some(monitor_state) = monitor_states.get(&backend) else {
        return true;
    };

    now.saturating_sub(monitor_state.last_refresh_ns) >= poll_interval_ns.max(1)
}

/// Publish device deltas from one backend snapshot.
pub(crate) fn publish_device_events_from_snapshot(
    runtime_state: &AudioRuntimeState,
    backend: AudioBackend,
    snapshot: &MonitorSnapshot,
    now: u64,
    source: AudioEventSource,
) {
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
            && active_native_device_publish_streams(runtime_state, backend)
                .iter()
                .any(|stream| {
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
            if active_native_device_publish_streams(runtime_state, backend)
                .iter()
                .any(|stream| {
                    event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_FORMAT_CHANGE)
                })
            {
                publish_device_event(
                    runtime_state,
                    AudioEventKind::DeviceFormatChanged,
                    now,
                    backend,
                    id.clone(),
                    source,
                );
            }

            if active_native_device_publish_streams(runtime_state, backend)
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
            && active_native_device_publish_streams(runtime_state, backend)
                .iter()
                .any(|stream| {
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
        && active_native_device_publish_streams(runtime_state, backend)
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
        && active_native_device_publish_streams(runtime_state, backend)
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
        && active_native_device_publish_streams(runtime_state, backend)
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

/// Refresh synthetic device events for one stream when polling is enabled.
pub(crate) fn refresh_device_events_for_stream(
    runtime_state: &AudioRuntimeState,
    stream: &Arc<AudioEventStream>,
    now: u64,
) -> RuntimeResult<()> {
    // ignore streams that do not request device tracking
    if !tracks_device_events(stream.options) {
        return Ok(());
    }

    // skip refresh when the baseline is still fresh
    if !device_monitor_refresh_due(
        runtime_state,
        stream.options.backend,
        stream.options.poll_interval_ns,
        now,
    ) {
        return Ok(());
    }

    let snapshot = monitor_snapshot(stream.options.backend)?;
    publish_device_events_from_snapshot(
        runtime_state,
        stream.options.backend,
        &snapshot,
        now,
        AudioEventSource::SyntheticPoll,
    );

    Ok(())
}

/// Publish one backend-native device snapshot diff to active subscriptions.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub(crate) fn publish_device_snapshot_native(backend: AudioBackend) {
    AudioMonitorServiceRegistry::publish_native_snapshot(backend);
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
