use super::access::os_state;
use super::handle::resolve_lifecycle_stream;
use super::*;

/// Read the current lifecycle state from runtime-owned OS state.
pub(crate) fn lifecycle_state(binding: &BindingCallContext) -> RuntimeResult<LifecycleState> {
    let runtime_state = os_state(binding)?;

    Ok(runtime_state.lifecycle_state())
}

/// Open one lifecycle event stream.
pub(crate) fn lifecycle_open(
    binding: &BindingCallContext,
) -> RuntimeResult<resource::LifecycleEventHandle> {
    let runtime_state = os_state(binding)?;
    let stream = Arc::new(LifecycleEventStream::default());
    let stream_id = runtime_state.insert_lifecycle_stream(stream);

    // stream registration comes before handle publication
    let entry = ResourceEntry::new(ResourceKind::LifecycleEvent)
        .with_label("os.lifecycle.event")
        .with_payload(stream_id);

    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::LifecycleEventHandle(handle))
}

/// Close one lifecycle event stream.
pub(crate) fn lifecycle_close(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    let runtime_state = os_state(binding)?;
    let removed =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_handle("unknown lifecycle event stream handle"));
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_handle("unknown lifecycle event stream handle"));
    };

    let Ok(stream_id) = payload.downcast::<u64>() else {
        return Err(invalid_handle("unknown lifecycle event stream handle"));
    };
    let Some(stream) = runtime_state.remove_lifecycle_stream(*stream_id) else {
        return Err(invalid_handle("unknown lifecycle event stream handle"));
    };

    stream.close();

    Ok(())
}

/// Poll one lifecycle event without blocking.
pub(crate) fn lifecycle_try_read(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<LifecycleEventValue> {
    binding.advance_wait_progress()?;

    let stream = resolve_lifecycle_stream(binding, handle)?;
    let Some(event) = stream.try_take() else {
        return Err(io_would_block(
            "destack.os.lifecycle.tryRead",
            "no lifecycle event is currently queued",
        ));
    };

    Ok(event)
}

/// Wait for one lifecycle event.
pub(crate) fn lifecycle_read(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<LifecycleEventValue> {
    let stream = resolve_lifecycle_stream(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    binding.wait_for_binding_result(
        "destack.os.lifecycle.read",
        "timed out waiting for lifecycle event",
        deadline_ns,
        || {
            if stream.is_closed() {
                return Err(invalid_handle("unknown lifecycle event stream handle"));
            }

            Ok(stream.try_take())
        },
        |duration| stream.wait_once(duration),
    )
}

/// Runtime-owned lifecycle event stream.
#[derive(Debug, Default)]
pub(crate) struct LifecycleEventStream {
    /// Shared event queue state for this stream.
    queue: RuntimeEventQueue<LifecycleEventValue>,
    /// Sequence number for the next event in this stream.
    next_sequence: AtomicU64,
}

impl PlatformOsState {
    /// Push one lifecycle event to every live stream.
    fn publish_lifecycle_event(
        &self,
        build: impl Fn(&LifecycleEventStream) -> LifecycleEventValue,
    ) {
        let lifecycle_streams = self.lifecycle_streams();

        for stream in lifecycle_streams {
            stream.push_event(build(&stream));
        }
    }

    /// Apply one host lifecycle transition.
    pub(crate) fn observe_lifecycle_transition(&self, state: HostLifecycleState) {
        let previous_host_state = *self.host_lifecycle_state.read();
        let next_state = lifecycle_state_from_host(state);

        if previous_host_state == state {
            return;
        }

        {
            let mut lifecycle_state = self.lifecycle_state.write();
            *lifecycle_state = next_state;
        }

        {
            let mut host_lifecycle_state = self.host_lifecycle_state.write();
            *host_lifecycle_state = state;
        }

        let Some(kind) = lifecycle_event_kind_for_transition(previous_host_state, state) else {
            return;
        };

        self.publish_lifecycle_event(|stream| {
            lifecycle_event_for_kind(kind, stream.next_metadata())
        });
    }

    /// Apply one host memory-pressure event.
    pub(crate) fn observe_memory_pressure(&self, level: HostMemoryPressureLevel) {
        let severity = match level {
            HostMemoryPressureLevel::Normal => return,
            HostMemoryPressureLevel::Warning => 1,
            HostMemoryPressureLevel::Critical => 2,
        };

        self.publish_lifecycle_event(|stream| {
            LifecycleEventValue::LifecycleLowMemoryEvent(LifecycleLowMemoryEventValue {
                kind: "lowMemory".to_string(),
                metadata: stream.next_metadata(),
                payload: LifecycleLowMemoryPayload { severity },
            })
        });
    }

    /// Apply one host power-mode event.
    pub(crate) fn observe_power_mode(&self, mode: HostPowerMode) {
        let enabled = matches!(mode, HostPowerMode::LowPower);

        self.publish_lifecycle_event(|stream| {
            LifecycleEventValue::LifecycleLowPowerModeChangedEvent(
                LifecycleLowPowerModeChangedEventValue {
                    kind: "lowPowerModeChanged".to_string(),
                    metadata: stream.next_metadata(),
                    payload: LifecycleLowPowerPayload { enabled },
                },
            )
        });
    }
}

impl LifecycleEventStream {
    /// Return the next metadata payload for this stream.
    fn next_metadata(&self) -> LifecycleEventMetadata {
        let timestamp_ns = monotonic_now_ns();
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);

        LifecycleEventMetadata {
            timestamp_ns,
            sequence,
        }
    }

    /// Push one event and wake blocked readers.
    fn push_event(&self, event: LifecycleEventValue) {
        self.queue.push(event);
    }

    /// Try to take one queued event.
    pub(crate) fn try_take(&self) -> Option<LifecycleEventValue> {
        self.queue.try_take()
    }

    /// Return whether this stream is closed.
    pub(crate) fn is_closed(&self) -> bool {
        self.queue.is_closed()
    }

    /// Close this stream and wake blocked readers.
    pub(crate) fn close(&self) {
        self.queue.close();
    }

    /// Wait once for queued events or timeout.
    pub(crate) fn wait_once(&self, duration: Duration) {
        self.queue.wait_once(duration);
    }
}
