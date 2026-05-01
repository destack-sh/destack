use super::{
    BorrowMode, CheckOptions, CheckPolicy, ExecutionMode, ExecutionOptions, ExternalCallPolicy,
    LimitOptions, PolicyOptions, TEST_MAX_INSTRUCTIONS, TEST_MAX_STACK_BYTES, TEST_MAX_STACK_DEPTH,
    TelemetryOptions, TrustPolicy,
};
use serde::{Deserialize, Serialize};

/// Configuration options for a VM isolate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
        options.apply_trust_policy(TrustPolicy::Trusted);
        options
    }

    /// Create options for testing with smaller limits.
    pub fn test() -> Self {
        // use strict settings with tighter resource limits
        let mut options = Self::default();
        options.execution.mode = ExecutionMode::Debug;
        options.policy.borrow_mode = BorrowMode::Strict;
        options.limits.max_stack_depth = TEST_MAX_STACK_DEPTH;
        options.limits.max_stack_bytes = TEST_MAX_STACK_BYTES;
        options.limits.max_instructions = Some(TEST_MAX_INSTRUCTIONS);
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
        // use comptime defaults with internal trust
        let mut options = Self::default();
        options.apply_trust_policy(TrustPolicy::Internal);
        options
    }

    /// Apply trust policy defaults to this isolate.
    pub fn apply_trust_policy(&mut self, trust_policy: TrustPolicy) {
        // set the trust policy
        self.policy.trust_policy = trust_policy;

        // apply untrusted defaults
        if matches!(trust_policy, TrustPolicy::Untrusted) {
            self.policy.borrow_mode = BorrowMode::Strict;
            self.policy.external_calls = ExternalCallPolicy::Protected;
            self.checks.bounds = CheckPolicy::Always;
            self.checks.null = CheckPolicy::Always;
            self.checks.enforce_reference_kinds = true;
            self.checks.enforce_reference_mutability = true;
        }
    }
}
