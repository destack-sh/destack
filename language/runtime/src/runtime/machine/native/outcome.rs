use destack_program::{Continuation, Value};

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
}
