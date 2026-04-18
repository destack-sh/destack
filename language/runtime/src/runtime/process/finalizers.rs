use std::mem;

use destack_core::{Capture, CaptureMode};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Finalizer callback for runtime-level teardown.
pub trait RuntimeFinalizer: Send {
    /// Finalize runtime-owned service state.
    fn finalize(self: Box<Self>);
}

impl<F> RuntimeFinalizer for F
where
    F: FnOnce() + Send + 'static,
{
    fn finalize(self: Box<Self>) {
        (*self)();
    }
}

/// Runtime-owned finalizer state.
enum RuntimeFinalizerState {
    /// Runtime is active and accepts new finalizers.
    Active(Vec<Box<dyn RuntimeFinalizer>>),
    /// Runtime teardown has completed.
    Finalized,
}

impl Default for RuntimeFinalizerState {
    fn default() -> Self {
        Self::Active(Vec::new())
    }
}

/// Materialized runtime-finalizer image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeFinalizersImage {
    /// Whether teardown already completed before capture.
    pub is_finalized: bool,
}

/// Runtime-owned finalizer registry.
pub struct RuntimeFinalizers {
    /// Mutable finalizer list and teardown lifecycle marker.
    state: Mutex<RuntimeFinalizerState>,
}

impl RuntimeFinalizers {
    /// Register one runtime-level finalizer callback.
    pub fn register(&self, finalizer: impl RuntimeFinalizer + 'static) {
        self.register_boxed(Box::new(finalizer));
    }

    /// Register one boxed runtime-level finalizer callback.
    pub fn register_boxed(&self, finalizer: Box<dyn RuntimeFinalizer>) {
        let mut state = self.state.lock();

        // queue finalizers while the runtime is active
        if let RuntimeFinalizerState::Active(entries) = &mut *state {
            entries.push(finalizer);
            return;
        }

        drop(state);

        // run late registrations immediately after teardown
        finalizer.finalize();
    }

    /// Run all registered finalizers exactly once.
    pub fn run_all(&self) {
        let finalizers = {
            let mut state = self.state.lock();

            // skip repeated teardown calls
            let previous = mem::replace(&mut *state, RuntimeFinalizerState::Finalized);
            let RuntimeFinalizerState::Active(finalizers) = previous else {
                return;
            };

            finalizers
        };

        // finalize in reverse registration order
        for finalizer in finalizers.into_iter().rev() {
            finalizer.finalize();
        }
    }

    /// Fork one quiescent runtime-finalizer registry.
    pub(crate) fn try_fork(&self) -> Option<Self> {
        let state = self.state.lock();

        // finalized registries stay finalized in the child
        if matches!(&*state, RuntimeFinalizerState::Finalized) {
            return Some(Self {
                state: Mutex::new(RuntimeFinalizerState::Finalized),
            });
        }

        // active finalizers are not clonable across branches
        let RuntimeFinalizerState::Active(entries) = &*state else {
            return None;
        };
        if !entries.is_empty() {
            return None;
        }

        Some(Self::default())
    }

    // capture image
    fn image(&self, _mode: CaptureMode) -> RuntimeResult<RuntimeFinalizersImage> {
        let state = self.state.lock();

        Ok(RuntimeFinalizersImage {
            is_finalized: matches!(&*state, RuntimeFinalizerState::Finalized),
        })
    }
}

impl Capture for RuntimeFinalizers {
    type Image = RuntimeFinalizersImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one finalizer image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one finalizer image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        let mut state = self.state.lock();
        *state = if image.is_finalized {
            RuntimeFinalizerState::Finalized
        } else {
            RuntimeFinalizerState::Active(Vec::new())
        };

        Ok(())
    }
}

impl Default for RuntimeFinalizers {
    fn default() -> Self {
        Self {
            state: Mutex::new(RuntimeFinalizerState::default()),
        }
    }
}

impl std::fmt::Debug for RuntimeFinalizers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeFinalizers").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::RuntimeFinalizers;

    /// Runs finalizers once in reverse registration order.
    #[test]
    fn test_run_all_runs_once_in_reverse_order() {
        let finalizers = RuntimeFinalizers::default();
        let first = Arc::new(AtomicUsize::new(0));
        let second = Arc::new(AtomicUsize::new(0));
        let sequence = Arc::new(AtomicUsize::new(0));

        let first_hit = Arc::clone(&first);
        let first_sequence = Arc::clone(&sequence);
        finalizers.register(move || {
            let order = first_sequence.fetch_add(1, Ordering::SeqCst);
            first_hit.store(order + 1, Ordering::SeqCst);
        });

        let second_hit = Arc::clone(&second);
        let second_sequence = Arc::clone(&sequence);
        finalizers.register(move || {
            let order = second_sequence.fetch_add(1, Ordering::SeqCst);
            second_hit.store(order + 1, Ordering::SeqCst);
        });

        finalizers.run_all();
        finalizers.run_all();

        assert_eq!(second.load(Ordering::SeqCst), 1);
        assert_eq!(first.load(Ordering::SeqCst), 2);
    }

    /// Runs late registrations immediately once teardown has completed.
    #[test]
    fn test_register_after_teardown_runs_immediately() {
        let finalizers = RuntimeFinalizers::default();
        let hits = Arc::new(AtomicUsize::new(0));

        finalizers.run_all();

        let late_hits = Arc::clone(&hits);
        finalizers.register(move || {
            late_hits.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }
}
