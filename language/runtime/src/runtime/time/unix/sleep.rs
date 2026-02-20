use std::time::Duration;

use crate::diagnostic::RuntimeResult;

/// Sleep on one host clock for one duration in nanoseconds.
pub(crate) fn host_sleep_nanos(duration: u64) -> RuntimeResult<()> {
    // skip zero-length host sleeps
    if duration == 0 {
        return Ok(());
    }

    // sleep on the host scheduler
    std::thread::sleep(Duration::from_nanos(duration));

    Ok(())
}
