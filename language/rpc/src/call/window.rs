use std::future::poll_fn;
use std::task::{Poll, Waker};

use parking_lot::{Condvar, Mutex};

/// Available send window for one stream direction.
#[derive(Debug)]
pub(crate) struct SendWindow {
    /// Mutable window state.
    state: Mutex<State>,
    /// Waiters blocked until capacity or closure.
    changed: Condvar,
}

impl SendWindow {
    /// Create one send window.
    pub(crate) fn new(available: u32) -> Self {
        Self {
            state: Mutex::new(State {
                available,
                is_closed: false,
                waker: None,
            }),
            changed: Condvar::new(),
        }
    }

    /// Wait for and consume one item of capacity.
    pub(crate) fn acquire(&self) -> bool {
        let mut state = self.state.lock();

        // wait until one item may be sent or the stream closes
        while state.available == 0 && !state.is_closed {
            self.changed.wait(&mut state);
        }
        if state.is_closed {
            return false;
        }

        state.available -= 1;

        true
    }

    /// Asynchronously wait for and consume one item of capacity.
    pub(crate) async fn acquire_async(&self) -> bool {
        poll_fn(|context| {
            let mut state = self.state.lock();
            if state.is_closed {
                return Poll::Ready(false);
            }
            if state.available > 0 {
                state.available -= 1;

                return Poll::Ready(true);
            }

            let should_replace = state
                .waker
                .as_ref()
                .is_none_or(|waker| !waker.will_wake(context.waker()));
            if should_replace {
                state.waker = Some(context.waker().clone());
            }

            Poll::Pending
        })
        .await
    }

    /// Consume one immediately available item of capacity.
    pub(crate) fn try_acquire(&self) -> bool {
        let mut state = self.state.lock();
        if state.is_closed || state.available == 0 {
            return false;
        }

        state.available -= 1;

        true
    }

    /// Add stream capacity from the peer when representable.
    pub(crate) fn try_update(&self, items: u32) -> bool {
        let mut state = self.state.lock();
        let Some(available) = state.available.checked_add(items) else {
            return false;
        };
        state.available = available;
        let waker = state.waker.take();
        self.changed.notify_all();
        drop(state);

        if let Some(waker) = waker {
            waker.wake();
        }

        true
    }

    /// Close the stream and release every waiter.
    pub(crate) fn close(&self) {
        let mut state = self.state.lock();
        state.is_closed = true;
        let waker = state.waker.take();
        self.changed.notify_all();
        drop(state);

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// Mutable send window state.
#[derive(Debug)]
struct State {
    /// Items currently permitted by the peer.
    available: u32,
    /// Whether no more items may be sent.
    is_closed: bool,
    /// Cooperative waiter for additional capacity.
    waker: Option<Waker>,
}
