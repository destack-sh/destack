use destack_session::ArtifactCancellation;

/// Cancellation retained while one workspace operation is active.
#[must_use = "dropping the guard cancels its workspace operation"]
#[derive(Debug)]
pub struct RunGuard {
    /// Artifact runs cancelled when the operation is abandoned.
    cancellations: Vec<ArtifactCancellation>,
}

impl RunGuard {
    /// Create a guard for scheduled artifact runs.
    pub(crate) fn new(cancellations: Vec<ArtifactCancellation>) -> Self {
        Self { cancellations }
    }

    /// Finish the operation without cancelling its artifact runs.
    pub fn finish(mut self) {
        self.cancellations.clear();
    }
}

impl Drop for RunGuard {
    /// Cancel unfinished workspace work.
    fn drop(&mut self) {
        for cancellation in self.cancellations.drain(..) {
            cancellation.cancel();
        }
    }
}
