#[cfg(feature = "parallel")]
use std::num::NonZero;
#[cfg(feature = "parallel")]
use std::thread;

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
