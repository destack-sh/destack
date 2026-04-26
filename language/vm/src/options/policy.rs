use serde::{Deserialize, Serialize};

use super::{DEFAULT_MAX_INSTRUCTIONS, DEFAULT_MAX_STACK_BYTES, DEFAULT_MAX_STACK_DEPTH};

/// Trust policy for runtime execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TrustPolicy {
    /// Untrusted code with strict limits and validation.
    #[default]
    Untrusted,
    /// Trusted code with relaxed limits.
    Trusted,
    /// Internal toolchain code with full privileges.
    Internal,
}

/// Policy for calling external functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExternalCallPolicy {
    /// Allow external calls with registered handlers.
    #[default]
    Allow,
    /// Allow external calls under crash protection when supported.
    Protected,
    /// Reject all external calls.
    Forbid,
}

/// Runtime capability policy for an isolate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RuntimePolicy {
    /// Allow all runtime features.
    #[default]
    Full,
    /// Disallow heap allocation and heap references.
    NoManaged,
    /// Disallow all runtime features.
    NoRuntime,
}

/// Borrow checking mode for owned and borrowed references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BorrowMode {
    /// Record violations as hints.
    #[default]
    Hint,
    /// Treat violations as runtime errors.
    Strict,
}

/// Runtime policy options for a VM isolate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyOptions {
    /// The trust policy for this isolate.
    pub trust_policy: TrustPolicy,
    /// The runtime capability policy for this isolate.
    pub runtime: RuntimePolicy,
    /// The borrow checking mode for this isolate.
    pub borrow_mode: BorrowMode,
    /// The external call policy for non interpreted functions.
    pub external_calls: ExternalCallPolicy,
}

/// Policy for runtime checks that can be disabled for speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
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

/// Runtime check configuration for a VM isolate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckOptions {
    /// The bounds check policy for field and element access.
    pub bounds: CheckPolicy,
    /// The null check policy for pointer dereferences.
    pub null: CheckPolicy,
    /// Enforce reference kind constraints for debug checks.
    pub enforce_reference_kinds: bool,
    /// Enforce reference mutability rules on stores for debug checks.
    pub enforce_reference_mutability: bool,
}

/// Resource limits for a VM isolate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOptions {
    /// The maximum call stack depth before a stack overflow error.
    pub max_stack_depth: usize,
    /// The maximum byte width of the VM stack arena.
    pub max_stack_bytes: usize,
    /// The maximum number of instructions to execute before timeout.
    /// None means no limit, use with caution.
    pub max_instructions: Option<u64>,
}

impl Default for LimitOptions {
    fn default() -> Self {
        // use default runtime limits
        Self {
            max_stack_depth: DEFAULT_MAX_STACK_DEPTH,
            max_stack_bytes: DEFAULT_MAX_STACK_BYTES,
            max_instructions: Some(DEFAULT_MAX_INSTRUCTIONS),
        }
    }
}
use super::ExecutionMode;
