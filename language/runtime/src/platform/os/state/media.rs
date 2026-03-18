use super::access::os_state;
use super::handle::resolve_media_stream;
use super::*;

/// Open one media watch stream.
pub(crate) fn media_watch_open(
    binding: &BindingCallContext,
    options: MediaWatchOptionsValue,
) -> RuntimeResult<resource::MediaWatchHandle> {
    require_media_capability(binding, "destack.os.media.watchOpen")?;

    let runtime_state = os_state(binding)?;
    let watch_id = runtime_state.next_media_watch_id();
    let stream = Arc::new(MediaWatchStream::new(options.clone()));

    // register before host open so early host events are not lost
    runtime_state.insert_media_watch(watch_id.clone(), stream);

    if let Err(error) = binding
        .host()
        .submit_operation(host_media::watch_open(watch_id.clone(), options))
    {
        runtime_state.remove_media_watch(watch_id.as_str());
        return Err(error);
    }

    let entry = ResourceEntry::new(ResourceKind::MediaWatch)
        .with_label("os.media.watch")
        .with_payload(watch_id);

    let handle = binding
        .agent()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::MediaWatchHandle(handle))
}

/// Close one media watch stream.
pub(crate) fn media_watch_close(
    binding: &BindingCallContext,
    handle: resource::MediaWatchHandle,
) -> RuntimeResult<()> {
    let runtime_state = os_state(binding)?;
    let watch_id = binding.agent().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<String>())
            .cloned()
    });
    let Some(watch_id) = watch_id.flatten() else {
        return Err(invalid_handle("unknown media watch stream handle"));
    };

    binding
        .host()
        .submit_operation(host_media::watch_close(watch_id.clone()))?;

    let removed =
        binding
            .agent()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_handle("unknown media watch stream handle"));
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_handle("unknown media watch stream handle"));
    };

    let Ok(watch_id) = payload.downcast::<String>() else {
        return Err(invalid_handle("unknown media watch stream handle"));
    };
    let Some(stream) = runtime_state.remove_media_watch(watch_id.as_str()) else {
        return Err(invalid_handle("unknown media watch stream handle"));
    };
    stream.close();

    Ok(())
}

/// Poll one media watch event without blocking.
pub(crate) fn media_watch_try_read(
    binding: &BindingCallContext,
    handle: resource::MediaWatchHandle,
) -> RuntimeResult<MediaEventValue> {
    binding.service_runtime_ingress()?;

    let stream = resolve_media_stream(binding, handle)?;
    let Some(event) = stream.try_take() else {
        return Err(io_would_block(
            "destack.os.media.watchTryRead",
            "no media event is currently queued",
        ));
    };

    Ok(event)
}

/// Wait for one media watch event.
pub(crate) fn media_watch_read(
    binding: &BindingCallContext,
    handle: resource::MediaWatchHandle,
    timeout_ns: u64,
) -> RuntimeResult<MediaEventValue> {
    let stream = resolve_media_stream(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    binding.wait_for_binding_result(
        "destack.os.media.watchRead",
        "timed out waiting for media event",
        deadline_ns,
        OS_READ_WAIT_SLICE_NS,
        || {
            if stream.is_closed() {
                return Err(invalid_handle("unknown media watch stream handle"));
            }

            Ok(stream.try_take())
        },
        |duration| stream.wait_once(duration),
    )
}

/// Require one live media read capability.
fn require_media_capability(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<()> {
    if binding
        .host()
        .has_host_capability(PlatformCapability::OsMediaRead)
    {
        return Ok(());
    }

    Err(not_supported(operation))
}

/// Runtime-owned media watch stream.
#[derive(Debug)]
pub(crate) struct MediaWatchStream {
    /// Open options that filter visible host events for this stream.
    options: MediaWatchOptionsValue,
    /// Shared event queue state for this stream.
    queue: RuntimeEventQueue<MediaEventValue>,
    /// Sequence number for the next event in this stream.
    next_sequence: AtomicU64,
}

impl PlatformOsState {
    /// Push one media event to the matching live stream.
    fn publish_media_event(
        &self,
        watch_id: &str,
        kind: HostMediaEventKind,
        asset: &MediaAssetSummaryValue,
    ) {
        let Some(stream) = self.media_watch(watch_id) else {
            return;
        };

        if !stream_accepts_media_event(stream.options(), kind) {
            return;
        }

        let metadata = stream.next_metadata();
        let event = match kind {
            HostMediaEventKind::Added => MediaEventValue::MediaAddedEvent(MediaAddedEventValue {
                kind: "added".to_string(),
                metadata,
                asset: asset.clone(),
            }),
            HostMediaEventKind::Updated => {
                MediaEventValue::MediaUpdatedEvent(MediaUpdatedEventValue {
                    kind: "updated".to_string(),
                    metadata,
                    asset: asset.clone(),
                })
            }
            HostMediaEventKind::Removed => {
                MediaEventValue::MediaRemovedEvent(MediaRemovedEventValue {
                    kind: "removed".to_string(),
                    metadata,
                    asset: asset.clone(),
                })
            }
        };

        stream.push_event(event);
    }

    /// Apply one host media event.
    pub(crate) fn observe_media_event(&self, event: &HostMediaEvent) {
        self.publish_media_event(event.watch_id.as_str(), event.kind, &event.asset);
    }
}

impl MediaWatchStream {
    /// Create one empty media watch stream.
    pub(crate) fn new(options: MediaWatchOptionsValue) -> Self {
        Self {
            options,
            queue: RuntimeEventQueue::default(),
            next_sequence: AtomicU64::new(0),
        }
    }

    /// Return the open options for this stream.
    fn options(&self) -> MediaWatchOptionsValue {
        self.options.clone()
    }

    /// Close this stream and wake blocked readers.
    pub(crate) fn close(&self) {
        self.queue.close();
    }

    /// Return whether this stream has already been closed.
    pub(crate) fn is_closed(&self) -> bool {
        self.queue.is_closed()
    }

    /// Try to take one queued media event.
    pub(crate) fn try_take(&self) -> Option<MediaEventValue> {
        self.queue.try_take()
    }

    /// Push one media event into this stream.
    fn push_event(&self, event: MediaEventValue) {
        self.queue.push(event);
    }

    /// Build one fresh stream-local media event metadata payload.
    fn next_metadata(&self) -> MediaEventMetadata {
        let timestamp_ns = monotonic_now_ns();
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);

        MediaEventMetadata {
            timestamp_ns,
            sequence,
        }
    }

    /// Wait once for queued media events or timeout.
    pub(crate) fn wait_once(&self, duration: Duration) {
        self.queue.wait_once(duration);
    }
}
