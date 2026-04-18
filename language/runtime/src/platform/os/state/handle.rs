use super::*;

/// Resolve one runtime-owned stream id from one resource payload.
fn resolve_stream_id(
    binding: &BindingCallContext,
    handle: resource::ResourceId,
    invalid_detail: &'static str,
) -> RuntimeResult<u64> {
    let resolved = binding.worker().resources.with_entry(handle, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<u64>())
            .copied()
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_handle(invalid_detail))
}

/// Resolve one runtime-owned watch id from one resource payload.
fn resolve_watch_id(
    binding: &BindingCallContext,
    handle: resource::ResourceId,
    invalid_detail: &'static str,
) -> RuntimeResult<String> {
    let resolved = binding.worker().resources.with_entry(handle, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<String>())
            .cloned()
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_handle(invalid_detail))
}

/// Resolve one lifecycle stream handle into one runtime-owned stream.
pub(super) fn resolve_lifecycle_stream(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<Arc<LifecycleEventStream>> {
    let stream_id = resolve_stream_id(binding, handle.0, "unknown lifecycle event stream handle")?;
    let runtime = os_state(binding)?;

    runtime
        .lifecycle_stream(stream_id)
        .ok_or_else(|| invalid_handle("unknown lifecycle event stream handle"))
}

/// Resolve one background stream handle into one runtime-owned stream.
pub(super) fn resolve_background_stream(
    binding: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<Arc<BackgroundEventStream>> {
    let stream_id = resolve_stream_id(binding, handle.0, "unknown background event stream handle")?;
    let runtime = os_state(binding)?;

    runtime
        .background_stream(stream_id)
        .ok_or_else(|| invalid_handle("unknown background event stream handle"))
}

/// Resolve one intent stream handle into one runtime-owned stream.
pub(super) fn resolve_intent_stream(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<Arc<IntentEventStream>> {
    let stream_id = resolve_stream_id(binding, handle.0, "unknown intent event stream handle")?;
    let runtime = os_state(binding)?;

    runtime
        .intent_stream(stream_id)
        .ok_or_else(|| invalid_handle("unknown intent event stream handle"))
}

/// Resolve one notification stream handle into one runtime-owned stream.
pub(super) fn resolve_notification_stream(
    binding: &BindingCallContext,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<Arc<NotificationEventStream>> {
    let stream_id = resolve_stream_id(
        binding,
        handle.0,
        "unknown notification event stream handle",
    )?;
    let runtime = os_state(binding)?;

    runtime
        .notification_stream(stream_id)
        .ok_or_else(|| invalid_handle("unknown notification event stream handle"))
}

/// Resolve one location watch handle into one runtime-owned stream.
pub(super) fn resolve_location_stream(
    binding: &BindingCallContext,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<Arc<LocationWatchStream>> {
    let watch_id = resolve_watch_id(binding, handle.0, "unknown location watch stream handle")?;
    let runtime = os_state(binding)?;

    runtime
        .location_watch(watch_id.as_str())
        .ok_or_else(|| invalid_handle("unknown location watch stream handle"))
}

/// Build one invalid-handle runtime error.
pub(crate) fn invalid_handle(detail: &'static str) -> Box<RuntimeError> {
    invalid_argument("handle", detail)
}
