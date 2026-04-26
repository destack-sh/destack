/// Shared runtime entry descriptor for all execution backends.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    /// The fully qualified entry name.
    name: String,
}

impl Entry {
    /// Create one entry descriptor by name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Return the fully qualified entry name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Output from one completed execution.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output<V> {
    /// Return value of the executed entrypoint.
    pub value: V,
}

/// Execution outcome produced by one backend.
#[derive(Debug)]
pub enum Outcome<C, O, Y = O> {
    /// Execution completed with a result.
    Completed {
        /// Completed execution output.
        output: Output<O>,
    },
    /// Execution yielded a continuation and resume value.
    Yielded {
        /// The continuation used to resume execution.
        continuation: C,
        /// The value yielded to the caller.
        value: Y,
    },
}
