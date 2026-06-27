use destack_program::{ContinuationImage, Value};

use super::Continuation;

/// Native execution outcome.
#[derive(Debug)]
pub enum Outcome {
    /// Native execution completed normally.
    Completed {
        /// The returned value.
        value: Value,
    },
    /// Native execution yielded a native continuation.
    Yielded {
        /// The continuation to resume.
        continuation: Continuation,
        /// The yielded value.
        value: Value,
    },
    /// Native execution deoptimized into a VM continuation image.
    Deoptimized {
        /// The VM continuation image.
        continuation: ContinuationImage,
    },
}
