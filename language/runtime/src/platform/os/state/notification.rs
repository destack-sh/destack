use super::access::os_state;
use super::handle::resolve_notification_stream;
use super::*;

/// Request host notification permission.
pub(crate) fn notification_request_permission(
    binding: &BindingCallContext,
) -> RuntimeResult<NotificationPermissionState> {
    let notification_state = binding
        .host()
        .submit_operation(host_notification::request_permission())?;
    let runtime_state = os_state(binding)?;
    let permission_state = match notification_state {
        NotificationPermissionState::Granted => PermissionState::Granted,
        NotificationPermissionState::Denied => PermissionState::Denied,
        NotificationPermissionState::Prompt => PermissionState::Prompt,
    };

    runtime_state.set_permission_state(Permission::Notifications, permission_state);

    Ok(notification_state)
}

/// Open one notification event stream.
pub(crate) fn notification_event_open(
    binding: &BindingCallContext,
    options: NotificationEventOpenOptionsValue,
) -> RuntimeResult<resource::NotificationEventHandle> {
    let runtime_state = os_state(binding)?;
    let stream = Arc::new(NotificationEventStream::new(options));
    let stream_id = runtime_state.insert_notification_stream(stream);

    let entry = ResourceEntry::new(ResourceKind::NotificationEvent)
        .with_label("os.notification.event")
        .with_payload(stream_id);

    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::NotificationEventHandle(handle))
}

/// Close one notification event stream.
pub(crate) fn notification_event_close(
    binding: &BindingCallContext,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<()> {
    let runtime_state = os_state(binding)?;
    let removed =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_handle("unknown notification event stream handle"));
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_handle("unknown notification event stream handle"));
    };

    let Ok(stream_id) = payload.downcast::<u64>() else {
        return Err(invalid_handle("unknown notification event stream handle"));
    };
    let Some(stream) = runtime_state.remove_notification_stream(*stream_id) else {
        return Err(invalid_handle("unknown notification event stream handle"));
    };

    stream.close();

    Ok(())
}

/// Poll one notification event without blocking.
pub(crate) fn notification_event_try_read(
    binding: &BindingCallContext,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<NotificationEventValue> {
    binding.advance_wait_progress()?;

    let stream = resolve_notification_stream(binding, handle)?;
    let Some(event) = stream.try_take() else {
        return Err(io_would_block(
            "destack.os.notification.event.tryRead",
            "no notification event is currently queued",
        ));
    };

    Ok(event)
}

/// Wait for one notification event.
pub(crate) fn notification_event_read(
    binding: &BindingCallContext,
    handle: resource::NotificationEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<NotificationEventValue> {
    // service ready ingress before waiting on the stream
    binding.advance_wait_progress()?;

    let stream = resolve_notification_stream(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    binding.wait_for_binding_result(
        "destack.os.notification.event.read",
        "timed out waiting for notification event",
        deadline_ns,
        || {
            if stream.is_closed() {
                return Err(invalid_handle("unknown notification event stream handle"));
            }

            Ok(stream.try_take())
        },
        |duration| stream.wait_once(duration),
    )
}

/// List registered notification categories through the host.
pub(crate) fn notification_category_list(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<NotificationCategoryValue>> {
    binding
        .host()
        .submit_operation(host_notification::category_list())
}

/// Register notification categories through the host.
pub(crate) fn notification_category_set(
    binding: &BindingCallContext,
    categories: Vec<NotificationCategoryValue>,
) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_notification::category_set(categories))
}

/// Post one notification immediately through the host.
pub(crate) fn notification_post(
    binding: &BindingCallContext,
    request: NotificationRequestValue,
) -> RuntimeResult<String> {
    binding
        .host()
        .submit_operation(host_notification::post(request))
}

/// Schedule one notification through the host.
pub(crate) fn notification_schedule(
    binding: &BindingCallContext,
    request: NotificationRequestValue,
) -> RuntimeResult<String> {
    binding
        .host()
        .submit_operation(host_notification::schedule(request))
}

/// Cancel one posted notification through the host.
pub(crate) fn notification_cancel(binding: &BindingCallContext, id: String) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_notification::cancel(id))
}

/// Cancel every posted notification through the host.
pub(crate) fn notification_cancel_all(binding: &BindingCallContext) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_notification::cancel_all())
}

/// List pending scheduled notifications through the host.
pub(crate) fn notification_pending_list(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    binding
        .host()
        .submit_operation(host_notification::pending_list())
}

/// Cancel one pending scheduled notification through the host.
pub(crate) fn notification_pending_cancel(
    binding: &BindingCallContext,
    id: String,
) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_notification::pending_cancel(id))
}

/// Cancel every pending scheduled notification through the host.
pub(crate) fn notification_pending_cancel_all(binding: &BindingCallContext) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_notification::pending_cancel_all())
}

/// Runtime-owned notification event stream.
#[derive(Debug)]
pub(crate) struct NotificationEventStream {
    /// Open options that filter which events are visible to this stream.
    options: NotificationEventOpenOptionsValue,
    /// Shared event queue state for this stream.
    queue: RuntimeEventQueue<NotificationEventValue>,
}

impl PlatformOsState {
    /// Push one notification event to every interested live stream.
    fn publish_notification_event(&self, event: &NotificationEventValue) {
        let notification_streams = self.notification_streams();

        for stream in notification_streams {
            if !stream_accepts_notification_event(stream.options(), event) {
                continue;
            }

            stream.push_event(event.clone());
        }
    }

    /// Apply one host notification event.
    pub(crate) fn observe_notification_event(&self, event: &HostNotificationEvent) {
        self.publish_notification_event(&event.event);
    }
}

impl NotificationEventStream {
    /// Create one empty notification event stream.
    pub(crate) fn new(options: NotificationEventOpenOptionsValue) -> Self {
        Self {
            options,
            queue: RuntimeEventQueue::default(),
        }
    }

    /// Return the open options for this stream.
    fn options(&self) -> NotificationEventOpenOptionsValue {
        self.options
    }

    /// Push one event and wake blocked readers.
    fn push_event(&self, event: NotificationEventValue) {
        self.queue.push(event);
    }

    /// Try to take one queued event.
    pub(crate) fn try_take(&self) -> Option<NotificationEventValue> {
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
