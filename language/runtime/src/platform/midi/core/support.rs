use crate::platform::core::{self as core_platform};

/// Return the current runtime monotonic timestamp.
pub(crate) fn binding_timestamp_now() -> u64 {
    core_platform::monotonic_now_ns()
}
