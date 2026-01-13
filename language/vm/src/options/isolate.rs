use super::{
    BorrowMode, CheckOptions, ExecutionMode, ExecutionOptions, ExternalCallPolicy, LimitOptions,
    PolicyOptions, TelemetryOptions,
};

/// Configuration options for a VM isolate.
#[derive(Debug, Clone, Default)]
pub struct IsolateOptions {
    /// Execution mode settings for this isolate.
    pub execution: ExecutionOptions,
    /// Runtime policies for borrowing, externals, and capabilities.
    pub policy: PolicyOptions,
    /// Runtime check configuration.
    pub checks: CheckOptions,
    /// Resource limits and execution budgets.
    pub limits: LimitOptions,
    /// Telemetry configuration for this isolate.
    pub telemetry: TelemetryOptions,
}

impl IsolateOptions {
    /// Create options with no step limit for trusted code.
    pub fn unbounded() -> Self {
        // use default options without instruction limits
        let mut options = Self::default();
        options.limits.max_instructions = None;
        options
    }

    /// Create options for testing with smaller limits.
    pub fn test() -> Self {
        // use strict settings with tighter resource limits
        let mut options = Self::default();
        options.execution.mode = ExecutionMode::Debug;
        options.policy.borrow_mode = BorrowMode::Strict;
        options.limits.max_stack_depth = 100;
        options.limits.max_heap_cells = 1000;
        options.limits.max_raw_cells = 1000;
        options.limits.max_instructions = Some(100_000);
        options.checks.enforce_reference_kinds = true;
        options.checks.enforce_reference_mutability = true;
        options
    }

    /// Create options for debug execution.
    pub fn debug() -> Self {
        // use strict runtime checks with protected externals
        let mut options = Self::default();
        options.execution.mode = ExecutionMode::Debug;
        options.policy.borrow_mode = BorrowMode::Strict;
        options.policy.external_calls = ExternalCallPolicy::Protected;
        options.checks.enforce_reference_kinds = true;
        options.checks.enforce_reference_mutability = true;
        options
    }

    /// Create options for comptime execution.
    pub fn comptime() -> Self {
        // use default comptime settings
        Self::default()
    }
}
