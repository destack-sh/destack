use std::collections::VecDeque;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform};
use crate::platform::midi::{
    MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED, MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
    MIDI_PORT_DIRECTION_FLAG_INPUT, MIDI_PORT_DIRECTION_FLAG_OUTPUT,
    MIDI_PORT_LIST_INCLUDE_DISCONNECTED, MIDI_PORT_LIST_INCLUDE_VIRTUAL, MidiEventOverflowPolicy,
    MidiEventSubscriptionFlags, MidiPortDirection, MidiPortDirectionFlags, MidiPortListFlags,
};
use crate::platform::resource::{ResourceEntry, ResourceId, ResourceKind};
use crate::runtime::BindingCallContext;

/// Resource label for one opened MIDI input session.
pub(crate) const MIDI_INPUT_RESOURCE_LABEL: &str = "midi.input.port";

/// Resource label for one opened MIDI output session.
pub(crate) const MIDI_OUTPUT_RESOURCE_LABEL: &str = "midi.output.port";

/// Resource label for one opened MIDI event subscription.
pub(crate) const MIDI_EVENT_RESOURCE_LABEL: &str = "midi.event";

/// Default input queue capacity when the caller leaves it zero.
const DEFAULT_INPUT_QUEUE_CAPACITY: usize = 1024;

/// Default event queue capacity when the caller leaves it zero.
const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;

/// Default synthetic event poll interval.
const DEFAULT_EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// One bounded queue shared with one callback producer.
pub(crate) struct SharedQueue<T> {
    /// Queue state and pending items.
    state: Mutex<SharedQueueState<T>>,

    /// Wake one waiting consumer.
    wake: Condvar,
}

/// Mutable queue state.
struct SharedQueueState<T> {
    /// Pending records.
    items: VecDeque<T>,

    /// Maximum retained items.
    capacity: usize,

    /// Number of dropped items.
    dropped_count: u64,
    /// Number of deferred overflow errors.
    overflow_error_count: u64,

    /// Whether the queue has been closed.
    is_closed: bool,
}

impl<T> SharedQueue<T> {
    /// Create one bounded queue.
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(SharedQueueState {
                items: VecDeque::new(),
                capacity: capacity.max(1),
                dropped_count: 0,
                overflow_error_count: 0,
                is_closed: false,
            }),
            wake: Condvar::new(),
        }
    }

    /// Push one item and drop the oldest item on overflow.
    pub(crate) fn push_drop_oldest(&self, item: T) {
        let mut state = self.state.lock();
        if state.is_closed {
            return;
        }

        // keep the newest callback records when the queue is saturated
        if state.items.len() >= state.capacity {
            state.items.pop_front();
            state.dropped_count = state.dropped_count.saturating_add(1);
        }

        state.items.push_back(item);
        self.wake.notify_one();
    }

    /// Push one item with one explicit overflow policy.
    pub(crate) fn push_with_overflow_policy(
        &self,
        item: T,
        policy: MidiEventOverflowPolicy,
    ) -> RuntimeResult<()> {
        let mut state = self.state.lock();
        if state.is_closed {
            return Ok(());
        }

        // event subscriptions may either drop or report overflow explicitly
        if state.items.len() >= state.capacity {
            match policy {
                MidiEventOverflowPolicy::DropOldest => {
                    state.items.pop_front();
                    state.dropped_count = state.dropped_count.saturating_add(1);
                }
                MidiEventOverflowPolicy::DropNewest => {
                    state.dropped_count = state.dropped_count.saturating_add(1);
                    return Ok(());
                }
                MidiEventOverflowPolicy::Error => {
                    state.overflow_error_count = state.overflow_error_count.saturating_add(1);
                    self.wake.notify_one();

                    return Ok(());
                }
            }
        }

        state.items.push_back(item);
        self.wake.notify_one();

        Ok(())
    }

    /// Take one deferred overflow error from the queue.
    pub(crate) fn take_overflow_error(&self, operation: &'static str) -> RuntimeResult<()> {
        let mut state = self.state.lock();
        if state.overflow_error_count == 0 {
            return Ok(());
        }

        let overflow_error_count = std::mem::take(&mut state.overflow_error_count);

        Err(core_platform::io_operation_error(
            operation,
            None,
            format!("midi event queue overflowed {overflow_error_count} time(s)"),
        ))
    }

    /// Pop one item immediately.
    pub(crate) fn try_pop(&self) -> Option<T> {
        let mut state = self.state.lock();

        state.items.pop_front()
    }

    /// Pop up to one batch immediately.
    pub(crate) fn try_pop_batch(&self, max_items: usize) -> Vec<T> {
        let mut state = self.state.lock();
        let count = max_items.min(state.items.len());

        state.items.drain(..count).collect()
    }

    /// Pop one item, waiting up to one timeout.
    pub(crate) fn pop_with_timeout(&self, timeout: Duration) -> Option<T> {
        if let Some(item) = self.try_pop() {
            return Some(item);
        }

        let deadline = Instant::now() + timeout;
        let mut state = self.state.lock();

        // sleep until either one producer arrives or the timeout expires
        while state.items.is_empty() && !state.is_closed {
            let now = Instant::now();
            if now >= deadline {
                return None;
            }

            self.wake.wait_for(&mut state, deadline - now);
        }

        state.items.pop_front()
    }

    /// Pop up to one batch, waiting up to one timeout.
    pub(crate) fn pop_batch_with_timeout(&self, max_items: usize, timeout: Duration) -> Vec<T> {
        let batch = self.try_pop_batch(max_items);
        if !batch.is_empty() {
            return batch;
        }

        let deadline = Instant::now() + timeout;
        let mut state = self.state.lock();

        // sleep until either one producer arrives or the timeout expires
        while state.items.is_empty() && !state.is_closed {
            let now = Instant::now();
            if now >= deadline {
                return Vec::new();
            }

            self.wake.wait_for(&mut state, deadline - now);
        }

        let count = max_items.min(state.items.len());

        state.items.drain(..count).collect()
    }

    /// Close the queue and wake waiters.
    pub(crate) fn close(&self) {
        let mut state = self.state.lock();
        state.is_closed = true;
        self.wake.notify_all();
    }

    /// Return the current dropped count.
    pub(crate) fn dropped_count(&self) -> u64 {
        self.state.lock().dropped_count
    }
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

/// Return one stable endpoint kind prefix.
pub(crate) fn endpoint_direction_name(direction: MidiPortDirection) -> &'static str {
    match direction {
        MidiPortDirection::Input => "input",
        MidiPortDirection::Output => "output",
    }
}

/// Return whether one direction mask includes one endpoint direction.
pub(crate) fn direction_mask_includes(
    mask: MidiPortDirectionFlags,
    direction: MidiPortDirection,
) -> bool {
    let direction_flag = match direction {
        MidiPortDirection::Input => MIDI_PORT_DIRECTION_FLAG_INPUT.0,
        MidiPortDirection::Output => MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
    };

    mask.0 & direction_flag != 0
}

/// Return port-list flags used to build one topology snapshot from event flags.
pub(crate) fn event_snapshot_list_flags(flags: MidiEventSubscriptionFlags) -> MidiPortListFlags {
    let mut list_flags = 0u32;

    // virtual endpoints are only included when the subscriber explicitly asks for them
    if flags.0 & MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL.0 != 0 {
        list_flags |= MIDI_PORT_LIST_INCLUDE_VIRTUAL.0;
    }

    // disconnected rows are opt-in so event snapshots stay tight by default
    if flags.0 & MIDI_EVENT_SUBSCRIPTION_INCLUDE_DISCONNECTED.0 != 0 {
        list_flags |= MIDI_PORT_LIST_INCLUDE_DISCONNECTED.0;
    }

    MidiPortListFlags(list_flags)
}

/// Build one runtime error for one missing resource handle.
pub(crate) fn missing_handle(
    operation: &'static str,
    handle_kind: &str,
    handle_id: ResourceId,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("{handle_kind} handle {} not found", handle_id.0),
    )
}

/// Read one typed session payload from one labeled resource entry.
pub(crate) fn read_labeled_resource_payload<R, F>(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    kind: ResourceKind,
    label: &'static str,
    operation: &'static str,
    handle_kind: &str,
    payload: F,
) -> RuntimeResult<R>
where
    F: FnOnce(&ResourceEntry) -> Option<R>,
{
    let mut payload = Some(payload);
    let session = binding
        .agent()
        .resources
        .with_entry(handle_id, |entry| {
            // reject entries from a different resource family
            if entry.kind != kind {
                return None;
            }

            // reject stale ids that point at a different label
            if entry.label.as_deref() != Some(label) {
                return None;
            }

            payload.take().and_then(|payload| payload(entry))
        })
        .flatten();

    match session {
        Some(session) => Ok(session),
        None => Err(missing_handle(operation, handle_kind, handle_id)),
    }
}

/// Remove one labeled resource entry or report one missing handle.
pub(crate) fn remove_labeled_resource(
    binding: &BindingCallContext,
    handle_id: ResourceId,
    operation: &'static str,
    handle_kind: &str,
) -> RuntimeResult<()> {
    let removed =
        binding
            .agent()
            .resources
            .remove(binding.world(), handle_id, Some(binding.engine()));

    match removed {
        Some(_) => Ok(()),
        None => Err(missing_handle(operation, handle_kind, handle_id)),
    }
}

/// Return the current runtime monotonic timestamp.
pub(crate) fn binding_timestamp_now() -> u64 {
    core_platform::monotonic_now_ns()
}

#[cfg(test)]
mod tests {
    use super::SharedQueue;
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::platform::midi::MidiEventOverflowPolicy;

    /// Report deferred overflow errors on the consumer side.
    #[test]
    fn test_shared_queue_defers_overflow_error_until_consumer_observes_it() {
        let queue = SharedQueue::new(1);

        queue
            .push_with_overflow_policy(1u32, MidiEventOverflowPolicy::Error)
            .expect("first queue push should succeed");
        queue
            .push_with_overflow_policy(2u32, MidiEventOverflowPolicy::Error)
            .expect("overflow should be deferred until the consumer checks the queue");

        let error = queue
            .take_overflow_error("destack.midi.event.read")
            .expect_err("consumer should observe the deferred overflow error");
        let error = error
            .platform_error()
            .expect("error should be a platform error");

        assert_eq!(error.code, PlatformErrorCode::Io);
    }
}
