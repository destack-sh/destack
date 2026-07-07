use destack_program::{Continuation, StopReason, Value};

/// Native execution outcome.
#[derive(Debug)]
pub enum Outcome {
    /// Native execution completed normally.
    Completed {
        /// The returned value.
        value: Value,
    },
    /// Native execution yielded a continuation.
    Yielded {
        /// The continuation to resume.
        continuation: Continuation,
        /// The yielded value.
        value: Value,
    },
    /// Native execution deoptimized into a continuation.
    Deoptimized {
        /// The continuation for VM fallback.
        continuation: Continuation,
    },
    /// Native execution stopped for host inspection.
    Stopped {
        /// The continuation to continue.
        continuation: Continuation,
        /// The reason execution stopped.
        reason: StopReason,
    },
}
