use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::MidiEventOverflowPolicy;

/// Default input queue capacity when the caller leaves it zero.
const DEFAULT_INPUT_QUEUE_CAPACITY: usize = 1024;
/// Default event queue capacity when the caller leaves it zero.
const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;
/// Default synthetic event poll interval.
const DEFAULT_EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) use crate::runtime::core::queue::BoundedQueue;

/// Push one event item with one explicit overflow policy.
pub(crate) fn push_event_with_overflow_policy<T>(
    queue: &BoundedQueue<T>,
    item: T,
    policy: MidiEventOverflowPolicy,
) -> RuntimeResult<()> {
    // event subscriptions may either drop or report overflow explicitly
    match policy {
        MidiEventOverflowPolicy::DropOldest => queue.push_drop_oldest(item),
        MidiEventOverflowPolicy::DropNewest => queue.push_drop_newest(item),
        MidiEventOverflowPolicy::Error => queue.defer_overflow(),
    }

    Ok(())
}

/// Surface one deferred event overflow error for one consumer operation.
pub(crate) fn take_event_overflow_error<T>(
    queue: &BoundedQueue<T>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let overflow_count = queue.take_deferred_overflow_count();
    if overflow_count == 0 {
        return Ok(());
    }

    Err(core_platform::io_operation_error(
        operation,
        None,
        format!("midi event queue overflowed {overflow_count} time(s)"),
    ))
}

/// Return the default queue capacity for one input session.
pub(crate) fn input_queue_capacity(queue_capacity: u32) -> usize {
    if queue_capacity == 0 {
        return DEFAULT_INPUT_QUEUE_CAPACITY;
    }

    queue_capacity as usize
}

/// Return the default poll interval for one event subscription.
pub(crate) fn event_poll_interval(poll_interval_ns: u64) -> Duration {
    if poll_interval_ns == 0 {
        return DEFAULT_EVENT_POLL_INTERVAL;
    }

    Duration::from_nanos(poll_interval_ns)
}

/// Return the default queue capacity for one event subscription.
pub(crate) fn event_queue_capacity(queue_capacity: u32) -> usize {
    if queue_capacity == 0 {
        return DEFAULT_EVENT_QUEUE_CAPACITY;
    }

    queue_capacity as usize
}

#[cfg(test)]
mod tests {
    use super::{BoundedQueue, push_event_with_overflow_policy, take_event_overflow_error};
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::platform::midi::MidiEventOverflowPolicy;

    /// Report deferred overflow errors on the consumer side.
    #[test]
    fn test_bounded_queue_defers_overflow_error_until_consumer_observes_it() {
        let queue = BoundedQueue::new(1);

        push_event_with_overflow_policy(&queue, 1u32, MidiEventOverflowPolicy::Error)
            .expect("first queue push should succeed");
        push_event_with_overflow_policy(&queue, 2u32, MidiEventOverflowPolicy::Error)
            .expect("overflow should be deferred until the consumer checks the queue");

        let error = take_event_overflow_error(&queue, "destack.midi.event.read")
            .expect_err("consumer should observe the deferred overflow error");
        let error = error
            .platform_error()
            .expect("error should be a platform error");

        assert_eq!(error.code, PlatformErrorCode::Io);
    }
}
