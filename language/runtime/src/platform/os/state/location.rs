use super::access::os_state;
use super::handle::resolve_location_stream;
use super::*;

/// Read whether host location services are enabled.
pub(crate) fn location_services_enabled(binding: &BindingCallContext) -> RuntimeResult<bool> {
    let _ = binding;

    Err(not_supported("destack.os.location.servicesEnabled"))
}

/// Read one cached host location sample.
pub(crate) fn location_last_known(
    binding: &BindingCallContext,
) -> RuntimeResult<LocationSampleValue> {
    let _ = binding;

    Err(not_supported("destack.os.location.lastKnown"))
}

/// Open one location watch stream.
pub(crate) fn location_watch_open(
    binding: &BindingCallContext,
    options: LocationWatchOptionsValue,
) -> RuntimeResult<resource::LocationWatchHandle> {
    let _ = binding;
    let _ = options;

    Err(not_supported("destack.os.location.watchOpen"))
}

/// Close one location watch stream.
pub(crate) fn location_watch_close(
    binding: &BindingCallContext,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<()> {
    let runtime_state = os_state(binding)?;
    let watch_id = binding.worker().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<String>())
            .cloned()
    });
    let Some(watch_id) = watch_id.flatten() else {
        return Err(invalid_handle("unknown location watch stream handle"));
    };

    binding
        .host()
        .submit_operation(host_location::watch_close(watch_id.clone()))?;

    let removed =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_handle("unknown location watch stream handle"));
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_handle("unknown location watch stream handle"));
    };

    let Ok(watch_id) = payload.downcast::<String>() else {
        return Err(invalid_handle("unknown location watch stream handle"));
    };
    let Some(stream) = runtime_state.remove_location_watch(watch_id.as_str()) else {
        return Err(invalid_handle("unknown location watch stream handle"));
    };
    stream.close();

    Ok(())
}

/// Poll one location sample without blocking.
pub(crate) fn location_watch_try_read(
    binding: &BindingCallContext,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<LocationSampleValue> {
    binding.advance_wait_progress()?;

    let stream = resolve_location_stream(binding, handle)?;
    let Some(sample) = stream.try_take() else {
        return Err(io_would_block(
            "destack.os.location.watchTryRead",
            "no location sample is currently queued",
        ));
    };

    Ok(sample)
}

/// Wait for one location sample.
pub(crate) fn location_watch_read(
    binding: &BindingCallContext,
    handle: resource::LocationWatchHandle,
    timeout_ns: u64,
) -> RuntimeResult<LocationSampleValue> {
    // service ready ingress before waiting on the stream
    binding.advance_wait_progress()?;

    let stream = resolve_location_stream(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    binding.wait_for_binding_result(
        "destack.os.location.watchRead",
        "timed out waiting for location sample",
        deadline_ns,
        || {
            if stream.is_closed() {
                return Err(invalid_handle("unknown location watch stream handle"));
            }

            Ok(stream.try_take())
        },
        |duration| stream.wait_once(duration),
    )
}

/// Runtime-owned location watch stream.
#[derive(Debug)]
pub(crate) struct LocationWatchStream {
    /// Shared event queue state for this stream.
    queue: RuntimeEventQueue<LocationSampleValue>,
}

impl PlatformOsState {
    /// Read the most recent location sample for this runtime.
    pub(crate) fn last_location_sample(&self) -> Option<LocationSampleValue> {
        *self.last_location_sample.read()
    }

    /// Push one location sample to the matching live stream.
    fn publish_location_event(&self, watch_id: &str, sample: &LocationSampleValue) {
        let Some(stream) = self.location_watch(watch_id) else {
            return;
        };

        stream.push_event(*sample);
    }

    /// Apply one host location event.
    pub(crate) fn observe_location_event(&self, event: &HostLocationEvent) {
        {
            let mut last_location_sample = self.last_location_sample.write();
            *last_location_sample = Some(event.sample);
        }

        self.publish_location_event(event.watch_id.as_str(), &event.sample);
    }
}

impl LocationWatchStream {
    /// Create one empty location watch stream.
    pub(crate) fn new() -> Self {
        Self {
            queue: RuntimeEventQueue::default(),
        }
    }

    /// Push one sample and wake blocked readers.
    fn push_event(&self, sample: LocationSampleValue) {
        self.queue.push(sample);
    }

    /// Try to take one queued sample.
    pub(crate) fn try_take(&self) -> Option<LocationSampleValue> {
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

    /// Wait once for queued samples or timeout.
    pub(crate) fn wait_once(&self, duration: Duration) {
        self.queue.wait_once(duration);
    }
}
