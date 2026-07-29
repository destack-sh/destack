use destack_session::ArtifactCancellation;

/// Cancels artifact runs when an LSP operation is dropped.
pub(super) struct ArtifactRunGuard {
    /// Cancellation access retained while the operation is active.
    cancellations: Vec<ArtifactCancellation>,
}

impl ArtifactRunGuard {
    /// Guard scheduled artifact runs.
    pub(super) fn new(cancellations: Vec<ArtifactCancellation>) -> Self {
        Self { cancellations }
    }

    /// Disable cancellation after every artifact run finishes.
    pub(super) fn disarm(&mut self) {
        self.cancellations.clear();
    }
}

impl Drop for ArtifactRunGuard {
    /// Cancel unfinished artifact work.
    fn drop(&mut self) {
        for cancellation in self.cancellations.drain(..) {
            cancellation.cancel();
        }
    }
}
