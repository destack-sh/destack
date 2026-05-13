use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostError;
use crate::host::time::{ClockId, ClockProperties};

/// Runtime operation name for host clock metadata.
const HOST_CLOCK_METADATA_OPERATION: &str = "runtime.time.host.clock.info";
/// Runtime operation name for host clock reads by clock id.
const HOST_CLOCK_NOW_OPERATION: &str = "runtime.time.host.clock.now_nanos";
/// Runtime operation name for host process CPU clock samples.
const HOST_CLOCK_PROCESS_CPU_OPERATION: &str = "runtime.time.host.clock.process_cpu_nanos";
/// Runtime operation name for host thread CPU clock samples.
const HOST_CLOCK_THREAD_CPU_OPERATION: &str = "runtime.time.host.clock.thread_cpu_nanos";
/// Return one unsupported-operation runtime error.
fn unsupported_operation(operation: &str) -> Box<RuntimeError> {
    RuntimeError::from(HostError::not_supported(operation)).boxed()
}

/// Query one host clock metadata snapshot.
pub(crate) fn host_clock_metadata(clock: ClockId) -> RuntimeResult<ClockProperties> {
    let _ = clock;

    Err(unsupported_operation(HOST_CLOCK_METADATA_OPERATION))
}

/// Query one host clock sample in nanoseconds.
pub(crate) fn host_now_nanos(clock: ClockId) -> RuntimeResult<u64> {
    let _ = clock;

    Err(unsupported_operation(HOST_CLOCK_NOW_OPERATION))
}

/// Query one host process CPU clock sample in nanoseconds.
pub(crate) fn host_process_cpu_nanos() -> RuntimeResult<u64> {
    Err(unsupported_operation(HOST_CLOCK_PROCESS_CPU_OPERATION))
}

/// Query one host thread CPU clock sample in nanoseconds.
pub(crate) fn host_thread_cpu_nanos() -> RuntimeResult<u64> {
    Err(unsupported_operation(HOST_CLOCK_THREAD_CPU_OPERATION))
}
