use super::access::os_state;
use super::handle::resolve_background_stream;
use super::*;

/// Read background scheduler status through the host.
pub(crate) fn background_status(
    binding: &BindingCallContext,
) -> RuntimeResult<BackgroundStatusValue> {
    binding.host().submit_operation(host_background::status())
}

/// List background task registrations through the host.
pub(crate) fn background_list(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<BackgroundTaskDescriptorValue>> {
    binding.host().submit_operation(host_background::list())
}

/// Register one background task through the host.
pub(crate) fn background_register(
    binding: &BindingCallContext,
    options: BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_background::register(options))
}

/// Unregister one background task through the host.
pub(crate) fn background_unregister(
    binding: &BindingCallContext,
    identifier: String,
) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_background::unregister(identifier))
}

/// Trigger one background task through the host test bridge.
pub(crate) fn background_trigger_test(
    binding: &BindingCallContext,
    identifier: String,
) -> RuntimeResult<bool> {
    binding
        .host()
        .submit_operation(host_background::trigger_test(identifier))
}

/// Complete one active background task execution through the host.
pub(crate) fn background_complete(
    binding: &BindingCallContext,
    execution_id: String,
    result: BackgroundTaskResultValue,
) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_background::complete(execution_id, result))
}

/// Open one background event stream.
pub(crate) fn background_event_open(
    binding: &BindingCallContext,
    options: BackgroundEventOpenOptionsValue,
) -> RuntimeResult<resource::BackgroundEventHandle> {
    let _ = binding;
    let _ = options;

    Err(not_supported("destack.os.background.event.open"))
}

/// Close one background event stream.
pub(crate) fn background_event_close(
    binding: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<()> {
    let runtime_state = os_state(binding)?;
    let removed =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_handle("unknown background event stream handle"));
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_handle("unknown background event stream handle"));
    };

    let Ok(stream_id) = payload.downcast::<u64>() else {
        return Err(invalid_handle("unknown background event stream handle"));
    };
    let Some(stream) = runtime_state.remove_background_stream(*stream_id) else {
        return Err(invalid_handle("unknown background event stream handle"));
    };

    stream.close();

    Ok(())
}

/// Poll one background event without blocking.
pub(crate) fn background_event_try_read(
    binding: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<BackgroundEventValue> {
    binding.advance_wait_progress()?;

    let stream = resolve_background_stream(binding, handle)?;
    let Some(event) = stream.try_take() else {
        return Err(io_would_block(
            "destack.os.background.event.tryRead",
            "no background event is currently queued",
        ));
    };

    Ok(event)
}

/// Wait for one background event.
pub(crate) fn background_event_read(
    binding: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<BackgroundEventValue> {
    // service ready ingress before waiting on the stream
    binding.advance_wait_progress()?;

    let stream = resolve_background_stream(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    binding.wait_for_binding_result(
        "destack.os.background.event.read",
        "timed out waiting for background event",
        deadline_ns,
        || {
            if stream.is_closed() {
                return Err(invalid_handle("unknown background event stream handle"));
            }

            Ok(stream.try_take())
        },
        |duration| stream.wait_once(duration),
    )
}

/// Runtime-owned background event stream.
#[derive(Debug)]
pub(crate) struct BackgroundEventStream {
    /// Open options that filter which events are visible to this stream.
    options: BackgroundEventOpenOptionsValue,
    /// Shared event queue state for this stream.
    queue: RuntimeEventQueue<BackgroundEventValue>,
}

impl PlatformOsState {
    /// Push one background event to every interested live stream.
    fn publish_background_event(&self, event: &BackgroundEventValue) {
        let background_streams = self.background_streams();
        for stream in background_streams {
            if !stream_accepts_background_event(stream.options(), event) {
                continue;
            }
            stream.push_event(event.clone());
        }
    }

    /// Apply one host background event.
    pub(crate) fn observe_background_event(&self, event: &HostBackgroundEvent) {
        self.publish_background_event(&event.event);
    }
}

impl BackgroundEventStream {
    /// Create one empty background event stream.
    pub(crate) fn new(options: BackgroundEventOpenOptionsValue) -> Self {
        Self {
            options,
            queue: RuntimeEventQueue::default(),
        }
    }

    /// Return the open options for this stream.
    fn options(&self) -> BackgroundEventOpenOptionsValue {
        self.options
    }

    /// Push one event and wake blocked readers.
    fn push_event(&self, event: BackgroundEventValue) {
        self.queue.push(event);
    }

    /// Try to take one queued event.
    pub(crate) fn try_take(&self) -> Option<BackgroundEventValue> {
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
