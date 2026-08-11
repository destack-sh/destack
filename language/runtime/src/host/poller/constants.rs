/// Kernel user-data value reserved for poller wake events.
pub(crate) const WAKE_TOKEN_BITS: u64 = u64::MAX;

/// Kernel user-data value reserved for poller timeout events.
pub(crate) const TIMEOUT_TOKEN_BITS: u64 = u64::MAX - 1;
