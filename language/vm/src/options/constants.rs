/// The default maximum call depth.
pub(super) const DEFAULT_MAX_FRAMES: usize = 1024;

/// The default native VM frame-stack byte limit.
#[cfg(not(target_arch = "wasm32"))]
pub(super) const DEFAULT_STACK_BYTES: usize = 1024 * 1024;

/// The default WebAssembly VM frame-stack byte limit.
#[cfg(target_arch = "wasm32")]
pub(super) const DEFAULT_STACK_BYTES: usize = 256 * 1024;

/// The default instruction budget.
pub(super) const DEFAULT_MAX_INSTRUCTIONS: u64 = 10_000_000;

/// The stack depth used by test machines.
pub(super) const TEST_MAX_FRAMES: usize = 100;

/// The stack byte budget used by test machines.
pub(super) const TEST_STACK_BYTES: usize = 1024 * 1024;

/// The instruction budget used by test machines.
pub(super) const TEST_MAX_INSTRUCTIONS: u64 = 100_000;
