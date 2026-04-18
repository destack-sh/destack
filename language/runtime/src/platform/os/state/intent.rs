use super::access::os_state;
use super::handle::resolve_intent_stream;
use super::*;

/// Open one intent event stream.
pub(crate) fn intent_open(
    binding: &BindingCallContext,
    options: IntentOpenOptions,
) -> RuntimeResult<resource::IntentHandle> {
    let runtime_state = os_state(binding)?;
    let stream = Arc::new(IntentEventStream::new(options));
    let stream_id = runtime_state.insert_intent_stream(stream);

    // stream registration comes before handle publication
    let entry = ResourceEntry::new(ResourceKind::Intent)
        .with_label("os.intent")
        .with_payload(stream_id);

    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    Ok(resource::IntentHandle(handle))
}

/// Close one intent event stream.
pub(crate) fn intent_close(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    let runtime_state = os_state(binding)?;
    let removed =
        binding
            .worker()
            .resources
            .remove(&binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_handle("unknown intent event stream handle"));
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_handle("unknown intent event stream handle"));
    };

    let Ok(stream_id) = payload.downcast::<u64>() else {
        return Err(invalid_handle("unknown intent event stream handle"));
    };
    let Some(stream) = runtime_state.remove_intent_stream(*stream_id) else {
        return Err(invalid_handle("unknown intent event stream handle"));
    };

    stream.close();

    Ok(())
}

/// Poll one intent event without blocking.
pub(crate) fn intent_try_read(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<(Arc<IntentEventStream>, IntentQueuedEvent)> {
    binding.advance_wait_progress()?;

    let stream = resolve_intent_stream(binding, handle)?;
    let Some(event) = stream.try_take() else {
        return Err(io_would_block(
            "destack.os.intent.tryRead",
            "no intent event is currently queued",
        ));
    };

    Ok((stream, event))
}

/// Wait for one intent event.
pub(crate) fn intent_read(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
    timeout_ns: u64,
) -> RuntimeResult<(Arc<IntentEventStream>, IntentQueuedEvent)> {
    // service ready ingress before waiting on the stream
    binding.advance_wait_progress()?;

    let stream = resolve_intent_stream(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    let event = binding.wait_for_binding_result(
        "destack.os.intent.read",
        "timed out waiting for intent event",
        deadline_ns,
        || {
            if stream.is_closed() {
                return Err(invalid_handle("unknown intent event stream handle"));
            }

            Ok(stream.try_take())
        },
        |duration| stream.wait_once(duration),
    )?;

    Ok((stream, event))
}

/// One queued intent event inside runtime-owned stream state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntentQueuedEvent {
    /// Host source package, bundle, or process identifier when available.
    pub(crate) source: Option<String>,
    /// Intent payload delivered by the host.
    pub(crate) payload: IntentQueuedPayload,
}

/// One queued intent payload inside runtime-owned stream state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IntentQueuedPayload {
    /// Open-url payload.
    OpenUrl {
        /// URL payload from the host.
        url: String,
    },
    /// Open-file payload.
    OpenFile {
        /// Path payload from the host.
        path: String,
        /// Normalized content type when provided by the host.
        content_type: Option<String>,
    },
    /// Share-text payload.
    ShareText {
        /// Shared text payload from the host.
        text: String,
        /// Normalized content type when provided by the host.
        content_type: Option<String>,
    },
    /// Share-files payload.
    ShareFiles {
        /// Shared file path payloads from the host.
        paths: Vec<String>,
        /// Normalized content type when provided by the host.
        content_type: Option<String>,
    },
    /// Custom-action payload.
    CustomAction {
        /// Action identifier from the host.
        action: String,
        /// URL payload when provided by the host.
        url: Option<String>,
        /// File path payloads when provided by the host.
        paths: Vec<String>,
        /// Shared text payload when provided by the host.
        text: Option<String>,
        /// Normalized content type when provided by the host.
        content_type: Option<String>,
    },
}

/// Runtime-owned intent event stream.
#[derive(Debug)]
pub(crate) struct IntentEventStream {
    /// Open options that filter which events are visible to this stream.
    options: IntentOpenOptions,
    /// Shared event queue state for this stream.
    queue: RuntimeEventQueue<IntentQueuedEvent>,
    /// Sequence number for the next event in this stream.
    next_sequence: AtomicU64,
}

impl PlatformOsState {
    /// Push one intent event to every interested live stream.
    fn publish_intent_event(&self, event: &IntentQueuedEvent) {
        let intent_streams = self.intent_streams();

        for stream in intent_streams {
            if !stream_accepts_payload(stream.options(), &event.payload) {
                continue;
            }

            stream.push_event(event.clone());
        }
    }

    /// Apply one host intent event.
    pub(crate) fn observe_intent_event(&self, event: &HostIntentEvent) {
        let queued_event = IntentQueuedEvent {
            source: event.source.clone(),
            payload: intent_payload_from_host(&event.payload),
        };

        self.publish_intent_event(&queued_event);
    }
}

impl IntentEventStream {
    /// Create one empty intent event stream.
    pub(crate) fn new(options: IntentOpenOptions) -> Self {
        Self {
            options,
            queue: RuntimeEventQueue::default(),
            next_sequence: AtomicU64::new(0),
        }
    }

    /// Return the open options for this stream.
    fn options(&self) -> IntentOpenOptions {
        self.options
    }

    /// Push one event and wake blocked readers.
    fn push_event(&self, event: IntentQueuedEvent) {
        self.queue.push(event);
    }

    /// Try to take one queued event.
    pub(crate) fn try_take(&self) -> Option<IntentQueuedEvent> {
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

    /// Return the next sequence number for this stream.
    pub(crate) fn next_sequence(&self) -> u64 {
        self.next_sequence.fetch_add(1, Ordering::Relaxed)
    }
}
