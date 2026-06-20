use destack_program::{MaterializedContinuation, Value};

use crate::Continuation;

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
    /// Native execution deoptimized into VM materialization.
    Deoptimized {
        /// The materialized VM continuation.
        materialization: MaterializedContinuation,
    },
}
