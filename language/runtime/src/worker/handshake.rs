use std::sync::atomic::{AtomicU32, Ordering};

/// Process-local requests observed cooperatively by one worker.
#[derive(Debug, Default)]
pub(crate) struct Handshake {
    /// Pending request bits.
    pending: AtomicU32,
}

/// One runtime request delivered at an execution poll.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Run pending local or shared collection work.
    Collect = 1 << 0,
    /// Retain execution for host inspection.
    Pause = 1 << 1,
    /// Continue retained execution through bytecode.
    Deoptimize = 1 << 2,
    /// Terminate the active runnable.
    Terminate = 1 << 3,
}

/// One set of runtime requests consumed by a worker.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RequestSet(u32);

impl Handshake {
    /// Create one handshake without pending requests.
    pub(crate) const fn new() -> Self {
        Self {
            pending: AtomicU32::new(0),
        }
    }

    /// Publish one request to this worker.
    pub(crate) fn request(&self, request: Request) {
        self.pending.fetch_or(request as u32, Ordering::Release);
    }

    /// Return whether this worker has pending requests.
    #[inline(always)]
    pub(crate) fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Acquire) != 0
    }

    /// Return the process-local request word address used by native code.
    pub(crate) fn address(&self) -> *const u32 {
        self.pending.as_ptr()
    }

    /// Consume every request published before this handshake.
    pub(crate) fn take(&self) -> RequestSet {
        RequestSet(self.pending.swap(0, Ordering::AcqRel))
    }
}

impl RequestSet {
    /// Return whether this set contains one request.
    pub(crate) const fn contains(self, request: Request) -> bool {
        self.0 & request as u32 != 0
    }

    /// Return whether this set contains no requests.
    pub(crate) const fn is_empty(self) -> bool {
        self.0 == 0
    }
}
