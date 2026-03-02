/// Execution telemetry produced when execution completes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EngineTelemetry {
    /// Total number of MIR instructions executed.
    pub mir_instructions_executed: u64,
    /// Total number of threaded instructions executed.
    pub threaded_instructions_executed: u64,
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
