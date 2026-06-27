/// Result of running executable code.
#[derive(Debug)]
pub enum Outcome<C, O, Y = O> {
    /// Execution completed with a result.
    Completed {
        /// The completed execution value.
        value: O,
    },
    /// Execution yielded a continuation and resume value.
    Yielded {
        /// The continuation used to resume execution.
        continuation: C,
        /// The value yielded to the caller.
        value: Y,
    },
}
