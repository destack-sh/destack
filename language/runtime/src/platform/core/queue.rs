use std::collections::VecDeque;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};

/// One bounded producer-consumer queue.
pub(crate) struct BoundedQueue<T> {
    /// Queue state and pending items.
    state: Mutex<BoundedQueueState<T>>,
    /// Wake one waiting consumer.
    wake: Condvar,
}

/// Mutable queue state.
struct BoundedQueueState<T> {
    /// Pending items.
    items: VecDeque<T>,
    /// Maximum retained items.
    capacity: usize,
    /// Number of dropped items.
    dropped_count: u64,
    /// Number of deferred overflow markers.
    deferred_overflow_count: u64,
    /// Whether the queue has been closed.
    is_closed: bool,
}

impl<T> BoundedQueue<T> {
    /// Create one bounded queue.
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(BoundedQueueState {
                items: VecDeque::new(),
                capacity: capacity.max(1),
                dropped_count: 0,
                deferred_overflow_count: 0,
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

    /// Push one item and drop the newest item on overflow.
    pub(crate) fn push_drop_newest(&self, item: T) {
        let mut state = self.state.lock();
        if state.is_closed {
            return;
        }

        // keep the existing queue contents when the newest item is discarded
        if state.items.len() >= state.capacity {
            state.dropped_count = state.dropped_count.saturating_add(1);
            return;
        }

        state.items.push_back(item);
        self.wake.notify_one();
    }

    /// Record one deferred overflow marker and wake consumers.
    pub(crate) fn defer_overflow(&self) {
        let mut state = self.state.lock();
        if state.is_closed {
            return;
        }

        state.deferred_overflow_count = state.deferred_overflow_count.saturating_add(1);
        self.wake.notify_one();
    }

    /// Take and clear the deferred overflow count.
    pub(crate) fn take_deferred_overflow_count(&self) -> u64 {
        let mut state = self.state.lock();

        std::mem::take(&mut state.deferred_overflow_count)
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

    /// Pop one item, waiting up to one timeout, or defer to one caller-supplied fallback.
    pub(crate) fn pop_with_timeout_or_else<E>(
        &self,
        timeout: Duration,
        on_empty: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        match self.pop_with_timeout(timeout) {
            Some(item) => Ok(item),
            None => on_empty(),
        }
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

    /// Pop up to one batch, waiting up to one timeout, or defer to one caller-supplied fallback.
    pub(crate) fn pop_batch_with_timeout_or_else<E>(
        &self,
        max_items: usize,
        timeout: Duration,
        on_empty: impl FnOnce() -> Result<Vec<T>, E>,
    ) -> Result<Vec<T>, E> {
        let items = self.pop_batch_with_timeout(max_items, timeout);
        if !items.is_empty() {
            return Ok(items);
        }

        on_empty()
    }

    /// Pop one item immediately or defer to one caller-supplied fallback.
    pub(crate) fn try_pop_or_else<E>(
        &self,
        on_empty: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        match self.try_pop() {
            Some(item) => Ok(item),
            None => on_empty(),
        }
    }

    /// Pop up to one batch immediately or defer to one caller-supplied fallback.
    pub(crate) fn try_pop_batch_or_else<E>(
        &self,
        max_items: usize,
        on_empty: impl FnOnce() -> Result<Vec<T>, E>,
    ) -> Result<Vec<T>, E> {
        let items = self.try_pop_batch(max_items);
        if !items.is_empty() {
            return Ok(items);
        }

        on_empty()
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::BoundedQueue;

    /// Count deferred overflow markers until one consumer observes them.
    #[test]
    fn test_bounded_queue_counts_deferred_overflow_markers() {
        let queue = BoundedQueue::<u32>::new(1);

        queue.defer_overflow();
        queue.defer_overflow();

        let overflow_count = queue.take_deferred_overflow_count();
        let overflow_count_after_clear = queue.take_deferred_overflow_count();

        assert_eq!(overflow_count, 2);
        assert_eq!(overflow_count_after_clear, 0);
    }

    /// Prefer queued items over one timeout fallback.
    #[test]
    fn test_bounded_queue_pop_with_timeout_or_else_returns_one_queued_item() {
        let queue = BoundedQueue::new(1);
        queue.push_drop_oldest(7u32);

        let item = queue
            .pop_with_timeout_or_else(Duration::from_nanos(0), || -> Result<u32, &'static str> {
                Err("queue should not be empty")
            })
            .expect("queued item should be returned");

        assert_eq!(item, 7);
    }

    /// Run the fallback once the queue stays empty through one timeout.
    #[test]
    fn test_bounded_queue_pop_batch_with_timeout_or_else_runs_the_fallback() {
        let queue = BoundedQueue::<u32>::new(1);

        let error = queue
            .pop_batch_with_timeout_or_else(
                4,
                Duration::from_nanos(0),
                || -> Result<Vec<u32>, &'static str> { Err("empty") },
            )
            .expect_err("empty queue should run the fallback");

        assert_eq!(error, "empty");
    }

    /// Prefer queued items over one immediate fallback.
    #[test]
    fn test_bounded_queue_try_pop_batch_or_else_returns_one_queued_batch() {
        let queue = BoundedQueue::new(4);
        queue.push_drop_oldest(3u32);
        queue.push_drop_oldest(5u32);

        let batch = queue
            .try_pop_batch_or_else(8, || -> Result<Vec<u32>, &'static str> {
                Err("queue should not be empty")
            })
            .expect("queued batch should be returned");

        assert_eq!(batch, vec![3, 5]);
    }
}
