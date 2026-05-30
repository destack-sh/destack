use crate::diagnostic::RuntimeResult;
use crate::host::HostEvent;

use super::HostQueue;

/// Host poll output containing queued events.
#[derive(Debug, Default)]
pub struct HostPollResult {
    /// Host events drained by one poll call.
    pub events: Vec<HostEvent>,
}

/// Runtime boundary for process-local host integration.
pub(crate) trait Host: std::fmt::Debug + Send + Sync {
    /// Return whether the current execution context is the process main context.
    fn is_process_main_context(&self) -> bool {
        false
    }

    /// Advance immediately ready host events without blocking.
    fn advance_events(&self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Collect host events that are ready to enter the runtime.
    fn collect_events(&self) -> RuntimeResult<Vec<HostEvent>> {
        Ok(Vec::new())
    }

    /// Return one host wall-clock sample in nanoseconds.
    fn wall_nanos(&self) -> u64;

    /// Return one host monotonic-clock sample in nanoseconds.
    fn mono_nanos(&self) -> u64;

    /// Sleep on the host for one duration in nanoseconds.
    fn sleep_nanos(&self, duration_nanos: u64);

    /// Sleep on the host until one wall-clock deadline in nanoseconds.
    fn sleep_until_wall_nanos(&self, deadline_nanos: u64);

    /// Fill one buffer with host entropy.
    fn fill_random_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()>;

    /// Try to fill one buffer with host entropy without blocking.
    fn try_fill_random_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()>;

    /// Return one host entropy u64.
    fn random_u64(&self) -> RuntimeResult<u64>;
}

/// Advance host-owned events into one queue.
pub(crate) fn advance_host_events(host: &dyn Host, queue: &HostQueue) -> RuntimeResult<()> {
    host.advance_events()?;
    let events = host.collect_events()?;
    queue.enqueue(events)?;

    Ok(())
}

/// Poll host events through one queue.
pub(crate) fn poll_host_events(
    host: &dyn Host,
    queue: &HostQueue,
    timeout_nanos: Option<u64>,
) -> RuntimeResult<HostPollResult> {
    advance_host_events(host, queue)?;

    Ok(HostPollResult {
        events: queue.poll_events(timeout_nanos)?,
    })
}
