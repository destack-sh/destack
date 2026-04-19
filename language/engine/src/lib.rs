mod frame;
mod safepoint;
pub use frame::*;
pub use safepoint::*;

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

/// Output from one execution step that completed normally.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunOutput<V> {
    /// Return value of the executed entrypoint.
    pub value: V,
    /// Run statistics payload.
    pub stats: RunStats,
    /// Number of managed allocations at end of execution.
    pub managed_allocation_count: usize,
    /// Number of raw allocations at end of execution.
    pub raw_allocation_count: usize,
}

/// Yield result from one suspended execution step.
#[derive(Debug)]
pub struct Yielded<C, V> {
    /// The continuation used to resume execution.
    pub continuation: C,
    /// The value yielded to the caller.
    pub value: V,
}

/// Execution outcome produced by one backend.
#[derive(Debug)]
pub enum RunOutcome<C, V> {
    /// Execution completed with a result.
    Completed {
        /// Completed execution output.
        output: RunOutput<V>,
    },
    /// Execution yielded a continuation and resume value.
    Yielded {
        /// Yield information for the suspended execution.
        yielded: Yielded<C, V>,
    },
}

/// Run statistics produced when execution completes.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunStats {
    /// Total number of MIR instructions executed.
    pub mir_instructions_executed: u64,
    /// Total number of lowered instructions executed.
    pub lowered_instructions_executed: u64,
    /// Total number of function calls made.
    pub calls_made: u64,
    /// Maximum call stack depth reached.
    pub max_stack_depth: usize,
    /// Number of heap allocations performed.
    pub heap_allocations: u64,
    /// Number of branch and switch instructions executed.
    pub branches: u64,
    /// Number of load instructions executed.
    pub loads: u64,
    /// Number of store instructions executed.
    pub stores: u64,
}
