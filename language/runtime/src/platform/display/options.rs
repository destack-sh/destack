use crate::platform::core as core_platform;
use crate::runtime::BindingCallContext;

/// Default queue capacity for display event streams.
const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;
/// Default wait-slice interval for blocking display event reads.
const DEFAULT_EVENT_WAIT_SLICE_NS: u64 = 10_000_000;

/// Return the configured default display event queue capacity.
pub(crate) fn default_event_queue_capacity(binding: &BindingCallContext) -> usize {
    let configured = binding.agent().options.display.event_queue_capacity;
    core_platform::option_u64_to_usize_or_min(configured, DEFAULT_EVENT_QUEUE_CAPACITY, 1)
}

/// Resolve queue capacity for one display event stream open request.
pub(crate) fn resolved_event_queue_capacity(
    binding: &BindingCallContext,
    requested_capacity: u32,
) -> usize {
    // fall back to the runtime default when the caller leaves capacity unset
    if requested_capacity == 0 {
        return default_event_queue_capacity(binding);
    }

    core_platform::u32_to_usize(requested_capacity)
}

/// Return the configured display event wait-slice interval in nanoseconds.
pub(crate) fn event_wait_slice_ns(binding: &BindingCallContext) -> u64 {
    let configured = binding.agent().options.display.stream_wait_slice_ns;
    core_platform::option_u64_or_min(configured, DEFAULT_EVENT_WAIT_SLICE_NS, 1)
}
