use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core::constants::{
    EVENT_SUBSCRIBE_BACKEND, EVENT_SUBSCRIBE_INTERRUPTION, EVENT_SUBSCRIBE_STREAM,
    host_monotonic_nanos,
};
use crate::platform::audio::core::error::resolve_stream_host_state;
use crate::platform::audio::core::model::{
    AudioEventStream, AudioStreamHostState, audio_stream_monitor_baseline,
};
use crate::platform::audio::core::runtime::{AudioRuntimeState, event_subscription_enabled};
use crate::platform::audio::core::stream::stream_state_snapshot;
use crate::platform::audio::{
    AudioEventKind, AudioEventSource, AudioStreamStateKind, AudioStreamStatusFlags,
};
use crate::platform::resource::{AudioStreamHandle, ResourceId};
use crate::runtime::BindingCallContext;

use super::core::publish_stream_event;

/// Return whether one stream monitor refresh is due.
fn stream_monitor_refresh_due(
    runtime_state: &AudioRuntimeState,
    stream: ResourceId,
    poll_interval_ns: u64,
    now: u64,
) -> bool {
    let monitor_states = runtime_state
        .stream_monitor_baselines
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some(monitor_state) = monitor_states.get(&stream) else {
        return true;
    };

    now.saturating_sub(monitor_state.last_refresh_ns) >= poll_interval_ns.max(1)
}

/// Refresh synthetic stream events for one stream subscription when polling is enabled.
pub(crate) fn refresh_stream_events_for_stream(
    ctx: &BindingCallContext,
    runtime_state: &AudioRuntimeState,
    stream: &Arc<AudioEventStream>,
    now: u64,
) -> RuntimeResult<()> {
    let Some(stream_handle) = stream.options.stream else {
        return Ok(());
    };

    // skip refresh when the baseline is still fresh
    if !stream_monitor_refresh_due(
        runtime_state,
        stream_handle.0,
        stream.options.poll_interval_ns,
        now,
    ) {
        return Ok(());
    }

    // drop the baseline when the target stream no longer exists
    let resolved_stream =
        match resolve_stream_host_state(ctx, stream_handle, "destack.audio.event.read") {
            Ok(resolved_stream) => resolved_stream,
            Err(_) => {
                let mut monitor_states = runtime_state
                    .stream_monitor_baselines
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                monitor_states.remove(&stream_handle.0);
                return Ok(());
            }
        };

    let stream_state = stream_state_snapshot(&resolved_stream);
    let stream_device_id = resolved_stream.device.id.clone();
    let mut monitor_states = runtime_state
        .stream_monitor_baselines
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some(previous_state) = monitor_states.get_mut(&stream_handle.0) else {
        monitor_states.insert(
            stream_handle.0,
            audio_stream_monitor_baseline(
                stream_state.state,
                stream_device_id,
                stream_state.xrun_count,
                now,
            ),
        );
        return Ok(());
    };

    // stream-state changes
    if let Some(old_state) = previous_state.previous_stream_state
        && old_state != stream_state.state
    {
        if event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_STREAM) {
            publish_stream_event(
                runtime_state,
                AudioEventKind::StreamStateChanged,
                now,
                resolved_stream.device.backend,
                stream_device_id.clone(),
                stream_state.status_flags,
                0,
                stream_handle,
                AudioEventSource::SyntheticPoll,
            );
        }

        // interruption transitions
        if event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_INTERRUPTION) {
            if old_state != AudioStreamStateKind::Interrupted
                && stream_state.state == AudioStreamStateKind::Interrupted
            {
                publish_stream_event(
                    runtime_state,
                    AudioEventKind::InterruptionBegan,
                    now,
                    resolved_stream.device.backend,
                    stream_device_id.clone(),
                    stream_state.status_flags,
                    0,
                    stream_handle,
                    AudioEventSource::SyntheticPoll,
                );
            } else if old_state == AudioStreamStateKind::Interrupted
                && stream_state.state != AudioStreamStateKind::Interrupted
            {
                publish_stream_event(
                    runtime_state,
                    AudioEventKind::InterruptionEnded,
                    now,
                    resolved_stream.device.backend,
                    stream_device_id.clone(),
                    stream_state.status_flags,
                    0,
                    stream_handle,
                    AudioEventSource::SyntheticPoll,
                );
            }
        }

        // backend failure transitions
        if event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_BACKEND) {
            if stream_state.state == AudioStreamStateKind::BackendDisconnected {
                publish_stream_event(
                    runtime_state,
                    AudioEventKind::BackendDisconnected,
                    now,
                    resolved_stream.device.backend,
                    stream_device_id.clone(),
                    stream_state.status_flags,
                    0,
                    stream_handle,
                    AudioEventSource::SyntheticPoll,
                );
            } else if stream_state.state == AudioStreamStateKind::DeviceLost {
                publish_stream_event(
                    runtime_state,
                    AudioEventKind::BackendReset,
                    now,
                    resolved_stream.device.backend,
                    stream_device_id.clone(),
                    stream_state.status_flags,
                    0,
                    stream_handle,
                    AudioEventSource::SyntheticPoll,
                );
            }
        }
    }

    // stream device reroutes
    if previous_state.previous_stream_device_id.as_ref() != Some(&stream_device_id)
        && event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_STREAM)
    {
        publish_stream_event(
            runtime_state,
            AudioEventKind::StreamDeviceChanged,
            now,
            resolved_stream.device.backend,
            stream_device_id.clone(),
            stream_state.status_flags,
            0,
            stream_handle,
            AudioEventSource::SyntheticPoll,
        );
    }

    // xrun deltas
    if stream_state.xrun_count > previous_state.previous_stream_xrun_count
        && event_subscription_enabled(stream.options, EVENT_SUBSCRIBE_STREAM)
    {
        let delta = stream_state
            .xrun_count
            .saturating_sub(previous_state.previous_stream_xrun_count);
        publish_stream_event(
            runtime_state,
            AudioEventKind::StreamXRun,
            now,
            resolved_stream.device.backend,
            stream_device_id.clone(),
            stream_state.status_flags,
            delta,
            stream_handle,
            AudioEventSource::SyntheticPoll,
        );
    }

    previous_state.previous_stream_state = Some(stream_state.state);
    previous_state.previous_stream_device_id = Some(stream_device_id);
    previous_state.previous_stream_xrun_count = stream_state.xrun_count;
    previous_state.last_refresh_ns = now;

    Ok(())
}

/// Publish one native stream event to matching active subscriptions.
pub(crate) fn publish_stream_event_native(
    stream_handle: AudioStreamHandle,
    stream: &AudioStreamHostState,
    kind: AudioEventKind,
    status_flags: AudioStreamStatusFlags,
    xrun_count_delta: u64,
) {
    let runtime_state = stream
        .runtime_owner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .and_then(std::sync::Weak::upgrade);
    let Some(runtime_state) = runtime_state else {
        return;
    };

    let timestamp_ns = host_monotonic_nanos();

    // keep the synthetic polling baseline aligned with native publications
    {
        let stream_state = stream_state_snapshot(stream);
        let stream_device_id = stream.device.id.clone();
        let mut monitor_states = runtime_state
            .stream_monitor_baselines
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        monitor_states.insert(
            stream_handle.0,
            audio_stream_monitor_baseline(
                stream_state.state,
                stream_device_id.clone(),
                stream_state.xrun_count,
                timestamp_ns,
            ),
        );
    }

    publish_stream_event(
        &runtime_state,
        kind,
        timestamp_ns,
        stream.device.backend,
        stream.device.id.clone(),
        status_flags,
        xrun_count_delta,
        stream_handle,
        AudioEventSource::Native,
    );
}
