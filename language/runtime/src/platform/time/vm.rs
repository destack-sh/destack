use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::time::{ClockId, ClockInfoVm, SleepClock};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.time.clock.info.
pub(super) fn destack_time_clock_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    clock: ClockId,
) -> RuntimeResult<ClockInfoVm> {
    let _ = clock;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.clock.info is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.clock.monoNs.
pub(super) fn destack_time_mono_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.clock.monoNs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.clock.nowNs.
pub(super) fn destack_time_now_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    clock: ClockId,
) -> RuntimeResult<u64> {
    let _ = clock;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.clock.nowNs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.clock.processCpuNs.
pub(super) fn destack_time_process_cpu_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.clock.processCpuNs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.clock.threadCpuNs.
pub(super) fn destack_time_thread_cpu_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.clock.threadCpuNs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.clock.wallNs.
pub(super) fn destack_time_wall_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.clock.wallNs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.sleep.ns.
pub(super) fn destack_time_sleep_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    duration: u64,
) -> RuntimeResult<()> {
    let _ = duration;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.sleep.ns is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.sleep.onNs.
pub(super) fn destack_time_sleep_on_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    duration: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    let _ = (duration, clock);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.sleep.onNs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.sleep.untilNs.
pub(super) fn destack_time_sleep_until_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    deadline: u64,
) -> RuntimeResult<()> {
    let _ = deadline;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.sleep.untilNs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.time.sleep.untilOnNs.
pub(super) fn destack_time_sleep_until_on_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    deadline: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    let _ = (deadline, clock);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.time.sleep.untilOnNs is not available in the VM yet",
    ))
    .boxed())
}
