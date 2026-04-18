use super::*;

/// Return the shared `platform.os` state for this binding.
pub(crate) fn os_state(binding: &BindingCallContext) -> RuntimeResult<PlatformOsState> {
    // resolve the shared OS state owner
    let state = resolve_live_state(binding);
    state.mark_active();

    // bootstrap it against the runtime host queue when available
    bootstrap_live_state(binding, &state)?;

    Ok(state)
}

/// Return the shared `platform.os` state for this binding.
fn resolve_live_state(binding: &BindingCallContext) -> PlatformOsState {
    binding.worker().platform_state.os.clone()
}

/// Bootstrap one shared `platform.os` state from runtime queue delivery.
fn bootstrap_live_state(
    binding: &BindingCallContext,
    state: &PlatformOsState,
) -> RuntimeResult<()> {
    let observer: Arc<dyn HostEventObserver> = Arc::new(state.clone());
    let host_session_id = binding.host().host_session_id();
    let platform = binding.host().platform();
    let queue = HostSessionRegistry::queue_for_session(host_session_id, platform);

    // lifecycle state can still function without queue catchup if ingress routing
    // is not registered yet, or if tests are dispatching host events directly
    match queue {
        Ok(queue) => state.reconcile_host_queue(&queue, &observer),
        Err(error) if is_missing_host_queue_error(&error) => Ok(()),
        Err(error) => Err(error),
    }
}

/// Return whether one queue lookup failed because no host queue exists yet.
fn is_missing_host_queue_error(error: &RuntimeError) -> bool {
    matches!(
        error,
        RuntimeError::Platform(platform_error)
            if platform_error.code == PlatformErrorCode::NotSupported
                && platform_error
                    .context
                    .as_ref()
                    .and_then(|context| context.feature.as_deref())
                    .is_some_and(|feature| feature.starts_with("runtime.host.queue."))
    )
}
