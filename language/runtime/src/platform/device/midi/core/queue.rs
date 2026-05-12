use std::time::Duration;

#[cfg(any(target_os = "linux", windows))]
use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::device::MidiEventOverflowPolicy;

/// Default input queue capacity when the caller leaves it zero.
const DEFAULT_INPUT_QUEUE_CAPACITY: usize = 1024;
/// Default event queue capacity when the caller leaves it zero.
const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;
/// Default synthetic event poll interval.
const DEFAULT_EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) use crate::platform::core::BoundedQueue;
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

#[cfg(any(target_os = "linux", windows))]
/// Surface one deferred terminal error from one shared backend slot.
pub(crate) fn surface_terminal_error<T>(
    operation: &'static str,
    terminal_error: &Mutex<Option<T>>,
    message: impl FnOnce(&T) -> String,
) -> RuntimeResult<()> {
    let terminal_error = terminal_error.lock();
    let Some(terminal_error) = terminal_error.as_ref() else {
        return Ok(());
    };

    Err(core_platform::io_operation_error(
        operation,
        None,
        message(terminal_error),
    ))
}

/// Poll one queued event after surfacing deferred overflow.
pub(crate) fn try_pop_queued_event<T>(
    queue: &BoundedQueue<T>,
    operation: &'static str,
) -> RuntimeResult<Option<T>> {
    take_event_overflow_error(queue, operation)?;

    Ok(queue.try_pop())
}

/// Poll one queued event batch after surfacing deferred overflow.
pub(crate) fn try_pop_queued_event_batch<T>(
    queue: &BoundedQueue<T>,
    max_items: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<T>> {
    take_event_overflow_error(queue, operation)?;

    Ok(queue.try_pop_batch(max_items))
}

/// Read one queued event after surfacing deferred overflow.
pub(crate) fn read_queued_event<T>(
    queue: &BoundedQueue<T>,
    timeout_ns: u64,
    operation: &'static str,
    empty_message: &'static str,
) -> RuntimeResult<T> {
    take_event_overflow_error(queue, operation)?;

    match queue.pop_with_timeout(Duration::from_nanos(timeout_ns)) {
        Some(item) => Ok(item),
        None => Err(core_platform::io_would_block(operation, empty_message)),
    }
}

/// Read one queued event batch after surfacing deferred overflow.
pub(crate) fn read_queued_event_batch<T>(
    queue: &BoundedQueue<T>,
    max_items: usize,
    timeout_ns: u64,
    operation: &'static str,
    empty_message: &'static str,
) -> RuntimeResult<Vec<T>> {
    take_event_overflow_error(queue, operation)?;

    let items = queue.pop_batch_with_timeout(max_items.max(1), Duration::from_nanos(timeout_ns));
    if items.is_empty() {
        return Err(core_platform::io_would_block(operation, empty_message));
    }

    Ok(items)
}

/// Require one queued event after polling or report would-block.
pub(crate) fn require_queued_event<T>(
    item: Option<T>,
    operation: &'static str,
    empty_message: &'static str,
) -> RuntimeResult<T> {
    match item {
        Some(item) => Ok(item),
        None => Err(core_platform::io_would_block(operation, empty_message)),
    }
}

/// Require one non-empty queued event batch or report would-block.
pub(crate) fn require_queued_event_batch<T>(
    items: Vec<T>,
    operation: &'static str,
    empty_message: &'static str,
) -> RuntimeResult<Vec<T>> {
    if items.is_empty() {
        return Err(core_platform::io_would_block(operation, empty_message));
    }

    Ok(items)
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

/// Read one queued item or surface one deferred backend failure before reporting would-block.
pub(crate) fn read_queued_item<T>(
    queue: &BoundedQueue<T>,
    timeout_ns: u64,
    operation: &'static str,
    empty_message: &'static str,
    on_empty: impl FnOnce() -> RuntimeResult<()>,
) -> RuntimeResult<T> {
    queue.pop_with_timeout_or_else(Duration::from_nanos(timeout_ns), || {
        on_empty()?;

        Err(core_platform::io_would_block(operation, empty_message))
    })
}

/// Read one queued batch or surface one deferred backend failure before reporting would-block.
pub(crate) fn read_queued_batch<T>(
    queue: &BoundedQueue<T>,
    max_items: usize,
    timeout_ns: u64,
    operation: &'static str,
    empty_message: &'static str,
    on_empty: impl FnOnce() -> RuntimeResult<()>,
) -> RuntimeResult<Vec<T>> {
    queue.pop_batch_with_timeout_or_else(max_items.max(1), Duration::from_nanos(timeout_ns), || {
        on_empty()?;

        Err(core_platform::io_would_block(operation, empty_message))
    })
}

/// Poll one queued item or surface one deferred backend failure before reporting would-block.
pub(crate) fn try_read_queued_item<T>(
    queue: &BoundedQueue<T>,
    operation: &'static str,
    empty_message: &'static str,
    on_empty: impl FnOnce() -> RuntimeResult<()>,
) -> RuntimeResult<T> {
    queue.try_pop_or_else(|| {
        on_empty()?;

        Err(core_platform::io_would_block(operation, empty_message))
    })
}

/// Poll one queued batch or surface one deferred backend failure before reporting would-block.
pub(crate) fn try_read_queued_batch<T>(
    queue: &BoundedQueue<T>,
    max_items: usize,
    operation: &'static str,
    empty_message: &'static str,
    on_empty: impl FnOnce() -> RuntimeResult<()>,
) -> RuntimeResult<Vec<T>> {
    queue.try_pop_batch_or_else(max_items.max(1), || {
        on_empty()?;

        Err(core_platform::io_would_block(operation, empty_message))
    })
}

#[cfg(test)]
mod tests {
    use parking_lot::Mutex;

    use crate::platform::core::{self as core_platform, BoundedQueue};
    use crate::platform::device::MidiEventOverflowPolicy;
    use crate::platform::diagnostic::PlatformErrorCode;

    use super::{
        push_event_with_overflow_policy, read_queued_batch, read_queued_event,
        read_queued_event_batch, read_queued_item, require_queued_event,
        require_queued_event_batch, take_event_overflow_error, try_read_queued_batch,
        try_read_queued_item,
    };

    /// Report deferred overflow errors on the consumer side.
    #[test]
    fn test_bounded_queue_defers_overflow_error_until_consumer_observes_it() {
        let queue = BoundedQueue::new(1);

        push_event_with_overflow_policy(&queue, 1u32, MidiEventOverflowPolicy::Error)
            .expect("first queue push should succeed");
        push_event_with_overflow_policy(&queue, 2u32, MidiEventOverflowPolicy::Error)
            .expect("overflow should be deferred until the consumer checks the queue");

        let error = take_event_overflow_error(&queue, "destack.device.midi.event.read")
            .expect_err("consumer should observe the deferred overflow error");
        let error = error
            .platform_error()
            .expect("error should be a platform error");

        assert_eq!(error.code, PlatformErrorCode::Io);
    }

    /// Surface deferred backend failures before reporting would-block on one empty queue.
    #[test]
    fn test_read_queued_item_surfaces_one_deferred_backend_failure() {
        let queue = BoundedQueue::<u32>::new(1);
        let terminal_error = Mutex::new(Some("backend disconnected".to_string()));

        let error = read_queued_item(
            &queue,
            0,
            "destack.device.midi.input.read",
            "midi input queue is empty",
            || {
                let terminal_error = terminal_error.lock();
                let terminal_error = terminal_error
                    .as_ref()
                    .expect("terminal error should be present");

                Err(core_platform::io_operation_error(
                    "destack.device.midi.input.read",
                    None,
                    terminal_error.clone(),
                ))
            },
        )
        .expect_err("empty queue should surface the deferred failure");
        let error = error
            .platform_error()
            .expect("error should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::Io);
        assert_eq!(error.message.as_deref(), Some("backend disconnected"));
    }

    /// Report would-block once the queue is empty and no deferred backend failure exists.
    #[test]
    fn test_try_read_queued_batch_reports_would_block_without_one_terminal_error() {
        let queue = BoundedQueue::<u32>::new(1);

        let error = try_read_queued_batch(
            &queue,
            4,
            "destack.device.midi.input.tryReadBatch",
            "midi input queue is empty",
            || Ok(()),
        )
        .expect_err("empty queue should report would-block");
        let error = error
            .platform_error()
            .expect("error should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::IoWouldBlock);
    }

    /// Return queued items before consulting any deferred backend failure hook.
    #[test]
    fn test_queue_read_helpers_prefer_queued_items_over_terminal_errors() {
        let queue = BoundedQueue::new(4);
        queue.push_drop_oldest(7u32);
        queue.push_drop_oldest(11u32);

        let item = try_read_queued_item(
            &queue,
            "destack.device.midi.input.tryRead",
            "midi input queue is empty",
            || panic!("queued item should short-circuit terminal failure handling"),
        )
        .expect("queued item should be returned");
        let batch = read_queued_batch(
            &queue,
            4,
            0,
            "destack.device.midi.input.readBatch",
            "midi input queue is empty",
            || panic!("queued batch should short-circuit terminal failure handling"),
        )
        .expect("queued batch should be returned");

        assert_eq!(item, 7);
        assert_eq!(batch, vec![11]);
    }

    /// Report would-block when one event queue stays empty through the timeout.
    #[test]
    fn test_read_queued_event_reports_would_block_when_empty() {
        let queue = BoundedQueue::<u32>::new(1);

        let error = read_queued_event(
            &queue,
            0,
            "destack.device.midi.event.read",
            "midi event queue is empty",
        )
        .expect_err("empty event queue should report would-block");
        let error = error
            .platform_error()
            .expect("error should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::IoWouldBlock);
    }

    /// Report would-block when one queued event batch stays empty.
    #[test]
    fn test_require_queued_event_batch_reports_would_block_when_empty() {
        let error = require_queued_event_batch::<u32>(
            Vec::new(),
            "destack.device.midi.event.tryReadBatch",
            "midi event queue is empty",
        )
        .expect_err("empty event batch should report would-block");
        let error = error
            .platform_error()
            .expect("error should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::IoWouldBlock);
    }

    /// Return queued events without reshaping them when present.
    #[test]
    fn test_require_queued_event_returns_the_present_item() {
        let item = require_queued_event(
            Some(17u32),
            "destack.device.midi.event.tryRead",
            "midi event queue is empty",
        )
        .expect("present event should be returned");
        let batch = read_queued_event_batch(
            &{
                let queue = BoundedQueue::new(2);
                queue.push_drop_oldest(1u32);
                queue.push_drop_oldest(2u32);
                queue
            },
            4,
            0,
            "destack.device.midi.event.readBatch",
            "midi event queue is empty",
        )
        .expect("present event batch should be returned");

        assert_eq!(item, 17);
        assert_eq!(batch, vec![1, 2]);
    }
}
