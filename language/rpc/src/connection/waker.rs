use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

/// Callback used to wake an embedded session host.
pub(super) type WakeHandler = Arc<dyn Fn() + Send + Sync + 'static>;

/// Coalesced wake notification for one embedded connection host.
#[derive(Clone, Default)]
pub(super) struct HostWaker {
    /// Installed host callback.
    handler: Arc<Mutex<Option<WakeHandler>>>,
    /// Whether one wake remains unacknowledged by the host.
    is_pending: Arc<AtomicBool>,
}

impl std::fmt::Debug for HostWaker {
    /// Format the visible host wake state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostWaker")
            .field("has_handler", &self.handler.lock().is_some())
            .field("is_pending", &self.is_pending)
            .finish()
    }
}

impl HostWaker {
    /// Install the callback invoked for the next pending or future wake.
    pub(super) fn set(&self, handler: WakeHandler) {
        *self.handler.lock() = Some(handler.clone());

        if self.is_pending.load(Ordering::Acquire) {
            handler();
        }
    }

    /// Remove the installed host callback.
    pub(super) fn clear(&self) {
        self.handler.lock().take();
        self.is_pending.store(false, Ordering::Release);
    }

    /// Acknowledge the previous wake before polling ready calls.
    pub(super) fn acknowledge(&self) {
        self.is_pending.store(false, Ordering::Release);
    }

    /// Wake the host once until it acknowledges the notification.
    pub(super) fn wake(&self) {
        if self.is_pending.swap(true, Ordering::AcqRel) {
            return;
        }

        let handler = self.handler.lock().clone();
        if let Some(handler) = handler {
            handler();
        }
    }
}
