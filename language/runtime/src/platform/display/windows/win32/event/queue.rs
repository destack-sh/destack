use super::*;

/// Convert one remaining timeout payload into a condition wait duration.
pub(super) fn wait_duration(remaining_ns: u64) -> Duration {
    Duration::from_nanos(remaining_ns)
}

/// Return the current host thread identifier.
pub(super) fn current_thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

/// Ensure one window-event stream operation is running on its owner thread.
pub(super) fn ensure_window_event_thread(
    binding: &WindowEventBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let current = current_thread_id();
    // evaluate this condition
    if current == binding.owner_thread_id {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        "handle",
        format!(
            "{operation} must run on owner thread {}, current thread is {current}",
            binding.owner_thread_id
        ),
    ))
}

/// Consume one latched overflow condition from one event queue state.
pub(super) fn consume_overflow_error<Record, State: EventQueueState<Record>>(
    state: &mut State,
    operation: &'static str,
) -> RuntimeResult<()> {
    // evaluate this condition
    if *state.overflow_error_pending() {
        *state.overflow_error_pending() = false;
        return Err(core_platform::io_busy(operation, "event queue overflowed"));
    }

    Ok(())
}

/// Pop one pending record from one event queue state.
pub(super) fn pop_pending_record<Record, State: EventQueueState<Record>>(
    state: &mut State,
    operation: &'static str,
    would_block_message: &'static str,
) -> RuntimeResult<Record> {
    consume_overflow_error(state, operation)?;

    let record = state.pending().pop_front();
    let Some(record) = record else {
        return Err(core_platform::io_would_block(
            operation,
            would_block_message,
        ));
    };

    Ok(record)
}

/// Pop one pending batch from one event queue state.
pub(super) fn pop_pending_batch<Record, State: EventQueueState<Record>>(
    state: &mut State,
    max_events: usize,
    operation: &'static str,
    would_block_message: &'static str,
) -> RuntimeResult<Vec<Record>> {
    consume_overflow_error(state, operation)?;

    let pending = state.pending();
    // evaluate this condition
    if pending.is_empty() {
        return Err(core_platform::io_would_block(
            operation,
            would_block_message,
        ));
    }

    let take = max_events.min(pending.len());
    let mut records = Vec::with_capacity(take);
    // iterate this sequence
    for _ in 0..take {
        // evaluate this condition
        if let Some(record) = pending.pop_front() {
            records.push(record);
        }
    }

    Ok(records)
}

/// Record metadata hooks shared by monitor and window queue records.
trait EventRecordMetadata {
    /// Write one stream sequence number into this record.
    fn set_sequence(&mut self, sequence: u64);

    /// Write one dropped-event counter into this record.
    fn set_dropped_count(&mut self, dropped_count: u64);
}

impl EventRecordMetadata for DisplayEventRecord {
    /// Write this event sequence value.
    fn set_sequence(&mut self, sequence: u64) {
        self.sequence = sequence;
    }

    /// Write this event dropped counter.
    fn set_dropped_count(&mut self, dropped_count: u64) {
        self.dropped_count = dropped_count;
    }
}

impl EventRecordMetadata for WindowEventRecord {
    /// Write this event sequence value.
    fn set_sequence(&mut self, sequence: u64) {
        self.sequence = sequence;
    }

    /// Write this event dropped counter.
    fn set_dropped_count(&mut self, dropped_count: u64) {
        self.dropped_count = dropped_count;
    }
}

/// Push one event record into one stream queue.
fn push_event<Record, State>(state: &mut State, mut event: Record)
where
    Record: EventRecordMetadata,
    State: EventQueueState<Record>,
{
    // iterate while this condition holds
    while state.pending().len() >= state.queue_capacity() {
        // resolve this variant
        match state.overflow_policy() {
            DisplayEventOverflowPolicy::DropOldest => {
                state.pending().pop_front();
                let dropped_count = state.dropped_count().saturating_add(1);
                *state.dropped_count() = dropped_count;
            }
            DisplayEventOverflowPolicy::DropNewest => {
                let dropped_count = state.dropped_count().saturating_add(1);
                *state.dropped_count() = dropped_count;
                return;
            }
            DisplayEventOverflowPolicy::Error => {
                let pending_len = state.pending().len() as u64;
                let dropped_count = state
                    .dropped_count()
                    .saturating_add(pending_len)
                    .saturating_add(1);
                *state.dropped_count() = dropped_count;
                state.pending().clear();
                *state.overflow_error_pending() = true;
                return;
            }
        }
    }

    let sequence = *state.next_sequence();
    let dropped_count = *state.dropped_count();
    event.set_sequence(sequence);
    event.set_dropped_count(dropped_count);
    *state.next_sequence() = sequence.saturating_add(1);
    state.pending().push_back(event);
}

/// Push one monitor-event record into one stream queue.
pub(super) fn push_monitor_event(state: &mut MonitorEventState, event: DisplayEventRecord) {
    push_event(state, event);
}

/// Push one window-event record into one stream queue.
pub(super) fn push_window_event(state: &mut WindowEventState, event: WindowEventRecord) {
    push_event(state, event);
}

/// Retain live subscriber bindings while removing one identity.
pub(super) fn retain_live_without_identity<T>(registry: &mut Vec<Weak<T>>, identity: usize) {
    registry.retain(|value| {
        let Some(active) = value.upgrade() else {
            return false;
        };

        Arc::as_ptr(&active) as usize != identity
    });
}

/// Collect one live subscriber list from one weak registry.
fn collect_live_subscribers<T>(registry: &mut Vec<Weak<T>>) -> Vec<Arc<T>> {
    // retain live weak entries and collect strong references
    let mut subscribers = Vec::with_capacity(registry.len());
    registry.retain(|value| {
        let Some(active) = value.upgrade() else {
            return false;
        };

        // keep this live subscriber and include it in output list
        subscribers.push(active);
        true
    });
    subscribers
}

/// Publish one monitor-event record to all active stream subscribers.
pub(super) fn publish_monitor_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    record: DisplayEventRecord,
) {
    let subscribers = {
        let mut registry = runtime_state
            .monitor_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        collect_live_subscribers(&mut registry)
    };

    // iterate this sequence
    for binding in subscribers {
        // evaluate this condition
        if !binding.filter.matches(&record) {
            continue;
        }

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        push_monitor_event(&mut state, record.clone());
        drop(state);
        binding.signal.notify_all();
    }
}

/// Publish one window-event record to all active stream subscribers.
pub(super) fn publish_window_event(
    runtime_state: &Arc<DisplayEventRuntimeState>,
    record: WindowEventRecord,
) {
    let subscribers = {
        let mut registry = runtime_state
            .window_event_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        collect_live_subscribers(&mut registry)
    };

    // iterate this sequence
    for binding in subscribers {
        // evaluate this condition
        if !binding.filter.matches(&record) {
            continue;
        }

        let mut state = binding
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        push_window_event(&mut state, record.clone());
        drop(state);
        binding.signal.notify_all();
    }
}
