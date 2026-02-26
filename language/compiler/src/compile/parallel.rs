#[cfg(feature = "parallel")]
use std::env;
#[cfg(feature = "parallel")]
use std::num::NonZero;
#[cfg(feature = "parallel")]
use std::thread;

/// Default compiler thread stack size in bytes.
#[cfg(feature = "parallel")]
const COMPILER_STACK_BYTES_DEFAULT: usize = 64 * 1024 * 1024;

/// Resolve the compiler worker count for this build.
pub(crate) fn default_workers() -> u16 {
    // parallel build: use available hardware threads
    #[cfg(feature = "parallel")]
    {
        thread::available_parallelism()
            .unwrap_or(NonZero::new(1).unwrap())
            .get() as u16
    }

    // non-parallel build: force a single worker
    #[cfg(not(feature = "parallel"))]
    {
        1
    }
}

/// Resolve the effective worker count from options.
pub(crate) fn effective_worker_count(configured_workers: u16) -> u16 {
    // parallel build: respect configured worker count
    #[cfg(feature = "parallel")]
    {
        configured_workers.max(1)
    }

    // non-parallel build: force a single worker
    #[cfg(not(feature = "parallel"))]
    {
        let _ = configured_workers;
        1
    }
}

/// Resolve the compiler thread stack size.
#[cfg(feature = "parallel")]
pub(crate) fn compiler_stack_bytes() -> usize {
    // prefer explicit compiler stack size, then fall back to rust min stack
    let explicit = env::var("DESTACK_COMPILER_STACK_BYTES").ok();
    let fallback = env::var("RUST_MIN_STACK").ok();
    explicit
        .or(fallback)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(COMPILER_STACK_BYTES_DEFAULT)
}
