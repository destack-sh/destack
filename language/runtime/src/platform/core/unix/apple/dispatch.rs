#[cfg(target_os = "macos")]
use std::mem::MaybeUninit;
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
use dispatch2::{DispatchQueue, DispatchRetained};
#[cfg(target_os = "macos")]
use objc2::Message;
#[cfg(target_os = "macos")]
use objc2::rc::Retained;

/// Synchronize one retained Objective-C object through one dispatch queue.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(crate) struct DispatchBound<T>(*mut Retained<T>);

#[cfg(target_os = "macos")]
unsafe impl<T> Send for DispatchBound<T> {}
#[cfg(target_os = "macos")]
unsafe impl<T> Sync for DispatchBound<T> {}

#[cfg(target_os = "macos")]
impl<T> DispatchBound<T> {
    /// Wrap one retained Objective-C object for queue-confined access.
    pub(crate) unsafe fn new(value: Retained<T>) -> Self {
        Self(Box::into_raw(Box::new(value)))
    }

    /// Clone one queue-confined Objective-C object reference on its owning queue.
    pub(crate) fn clone_on(&self, queue: &DispatchQueue) -> Self
    where
        T: Message,
    {
        self.dispatch_on(queue, |value| unsafe { Self::new(value.retain()) })
    }

    /// Access the wrapped object on one specific dispatch queue.
    pub(crate) fn dispatch_on<F, R>(&self, queue: &DispatchQueue, callback: F) -> R
    where
        F: FnOnce(&T) -> R + Send,
        R: Send,
    {
        let mut result = MaybeUninit::uninit();

        queue.exec_sync(|| {
            result.write(callback(unsafe { self.get_unchecked() }));
        });

        unsafe { result.assume_init() }
    }

    /// Borrow the wrapped object while already executing on the owning queue.
    pub(crate) unsafe fn get_unchecked(&self) -> &T {
        unsafe { &*self.0 }
    }
}

#[cfg(target_os = "macos")]
impl<T> Drop for DispatchBound<T> {
    /// Release the retained Objective-C object allocation.
    fn drop(&mut self) {
        let boxed = unsafe { Box::from_raw(self.0) };

        drop(boxed);
    }
}

/// Return the shared serial dispatch queue for Apple host integrations.
#[cfg(target_os = "macos")]
pub(crate) fn apple_dispatch_queue() -> &'static DispatchQueue {
    static DISPATCH_QUEUE: OnceLock<dispatch2::DispatchRetained<DispatchQueue>> = OnceLock::new();

    DISPATCH_QUEUE.get_or_init(|| {
        let utility = DispatchQueue::global_queue(
            dispatch2::GlobalQueueIdentifier::QualityOfService(dispatch2::DispatchQoS::Utility),
        );

        apple_serial_dispatch_queue("DestackApple", Some(&utility))
    })
}

/// Create one named serial dispatch queue for Apple host integrations.
#[cfg(target_os = "macos")]
pub(crate) fn apple_serial_dispatch_queue(
    label: &'static str,
    target: Option<&DispatchQueue>,
) -> DispatchRetained<DispatchQueue> {
    DispatchQueue::new_with_target(label, dispatch2::DispatchQueueAttr::SERIAL, target)
}
