/// The default maximum call depth for untrusted execution.
pub(super) const DEFAULT_MAX_STACK_DEPTH: usize = 1024;

/// The default maximum VM stack byte size for untrusted execution.
pub(super) const DEFAULT_MAX_STACK_BYTES: usize = 8 * 1024 * 1024;

/// The default instruction budget for untrusted execution.
pub(super) const DEFAULT_MAX_INSTRUCTIONS: u64 = 10_000_000;

/// The stack depth used by test isolates.
pub(super) const TEST_MAX_STACK_DEPTH: usize = 100;

/// The stack byte budget used by test isolates.
pub(super) const TEST_MAX_STACK_BYTES: usize = 1024 * 1024;

/// The instruction budget used by test isolates.
pub(super) const TEST_MAX_INSTRUCTIONS: u64 = 100_000;
