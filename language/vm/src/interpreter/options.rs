/// Execution role for the VM interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionMode {
    /// Evaluate compile time blocks and expressions.
    #[default]
    Comptime,
    /// Run under the debugger with introspection enabled.
    Debug,
    /// Resume execution after deoptimization from native code.
    Deopt,
    /// Execute as a general runtime fallback.
    Runtime,
}

impl ExecutionMode {
    /// Report whether this mode should enable debug checks.
    pub fn is_debug(self) -> bool {
        matches!(self, Self::Debug)
    }
}

/// Policy for runtime checks that can be disabled for speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckPolicy {
    /// Always enforce the check.
    #[default]
    Always,
    /// Enforce the check only in debug mode.
    Debug,
    /// Never enforce the check.
    Never,
}

impl CheckPolicy {
    /// Report whether the check should run under the given execution mode.
    pub fn is_enabled_for(self, mode: ExecutionMode) -> bool {
        // capture the debug state
        let is_debug = mode.is_debug();

        // select the policy behavior
        match self {
            Self::Always => true,
            Self::Debug => is_debug,
            Self::Never => false,
        }
    }
}

/// Policy for calling external functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExternalCallPolicy {
    /// Allow external calls with registered handlers.
    #[default]
    Allow,
    /// Allow external calls under crash protection when supported.
    Protected,
    /// Reject all external calls.
    Forbid,
}

/// Runtime capability policy for the interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimePolicy {
    /// Allow all runtime features.
    #[default]
    Full,
    /// Disallow managed heap allocation and managed references.
    NoManaged,
    /// Disallow all runtime features.
    NoRuntime,
}

/// Borrow checking mode for owned and borrowed references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorrowMode {
    /// Record violations as hints.
    #[default]
    Hint,
    /// Treat violations as runtime errors.
    Strict,
}

/// Configuration options for the VM interpreter.
#[derive(Debug, Clone)]
pub struct MachineOptions {
    /// The execution role for this interpreter instance.
    pub execution_mode: ExecutionMode,

    /// The runtime capability policy for this interpreter instance.
    pub runtime_policy: RuntimePolicy,

    /// The borrow checking mode for owned and borrowed references.
    pub borrow_mode: BorrowMode,

    /// The bounds check policy for aggregate and array access.
    pub bounds_checks: CheckPolicy,

    /// The null check policy for pointer dereferences.
    pub null_checks: CheckPolicy,

    /// The external call policy for non interpreted functions.
    pub external_calls: ExternalCallPolicy,

    /// The maximum call stack depth before a stack overflow error.
    /// Default is 1024.
    pub max_stack_depth: usize,

    /// The maximum number of heap cells before allocation fails.
    /// Default is 100_000, roughly 10MB depending on cell size.
    pub max_heap_cells: usize,

    /// The maximum number of instructions to execute before timeout.
    /// None means no limit, use with caution.
    pub max_instructions: Option<u64>,

    /// Enforce reference kind constraints for debug checks.
    pub enforce_reference_kinds: bool,

    /// Enforce reference mutability rules on stores for debug checks.
    pub enforce_reference_mutability: bool,

    /// Collect execution statistics during runtime.
    pub collect_stats: bool,
}

impl Default for MachineOptions {
    fn default() -> Self {
        // default vm limits and policies
        Self {
            execution_mode: ExecutionMode::default(),
            runtime_policy: RuntimePolicy::default(),
            borrow_mode: BorrowMode::default(),
            bounds_checks: CheckPolicy::default(),
            null_checks: CheckPolicy::default(),
            external_calls: ExternalCallPolicy::default(),
            max_stack_depth: 1024,
            max_heap_cells: 100_000,
            max_instructions: Some(10_000_000),
            enforce_reference_kinds: false,
            enforce_reference_mutability: false,
            collect_stats: true,
        }
    }
}

impl MachineOptions {
    /// Create options with no step limit for trusted code.
    pub fn unbounded() -> Self {
        // use default options without instruction limits
        Self {
            max_instructions: None,
            ..Self::default()
        }
    }

    /// Create options for testing with smaller limits.
    pub fn test() -> Self {
        // use strict settings with tighter resource limits
        Self {
            execution_mode: ExecutionMode::Debug,
            borrow_mode: BorrowMode::Strict,
            max_stack_depth: 100,
            max_heap_cells: 1000,
            max_instructions: Some(100_000),
            enforce_reference_kinds: true,
            enforce_reference_mutability: true,
            ..Self::default()
        }
    }

    /// Create options for debug execution.
    pub fn debug() -> Self {
        // use strict runtime checks with protected externals
        Self {
            execution_mode: ExecutionMode::Debug,
            borrow_mode: BorrowMode::Strict,
            external_calls: ExternalCallPolicy::Protected,
            enforce_reference_kinds: true,
            enforce_reference_mutability: true,
            ..Self::default()
        }
    }

    /// Create options for comptime execution.
    pub fn comptime() -> Self {
        // use default comptime settings
        Self::default()
    }
}
