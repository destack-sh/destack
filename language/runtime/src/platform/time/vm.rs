use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::time::{ClockId, ClockInfoVm, ClockSource, SleepClock};
use crate::runtime::RuntimeCallContext;

/// Return wall clock time in nanoseconds.
pub fn destack_time_wall_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Ok(runtime.runtime().time.wall_nanos())
}

/// Return monotonic time in nanoseconds.
pub fn destack_time_mono_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Ok(runtime.runtime().time.mono_nanos())
}

/// Return clock metadata.
pub fn destack_time_clock_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    clock: ClockId,
) -> RuntimeResult<ClockInfoVm> {
    let info = match clock {
        ClockId::Wall => ClockInfoVm {
            id: ClockId::Wall,
            source: ClockSource::Realtime,
            resolution_ns: 1,
            is_monotonic: false,
        },
        ClockId::Monotonic => ClockInfoVm {
            id: ClockId::Monotonic,
            source: ClockSource::Monotonic,
            resolution_ns: 1,
            is_monotonic: true,
        },
        ClockId::ProcessCpu => ClockInfoVm {
            id: ClockId::ProcessCpu,
            source: ClockSource::PerformanceCounter,
            resolution_ns: 1,
            is_monotonic: true,
        },
        ClockId::ThreadCpu => ClockInfoVm {
            id: ClockId::ThreadCpu,
            source: ClockSource::PerformanceCounter,
            resolution_ns: 1,
            is_monotonic: true,
        },
        ClockId::Boot => ClockInfoVm {
            id: ClockId::Boot,
            source: ClockSource::Monotonic,
            resolution_ns: 1,
            is_monotonic: true,
        },
        ClockId::MonotonicRaw => ClockInfoVm {
            id: ClockId::MonotonicRaw,
            source: ClockSource::Monotonic,
            resolution_ns: 1,
            is_monotonic: true,
        },
    };

    Ok(info)
}

/// Return the current nanoseconds value for the selected clock.
pub fn destack_time_now_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    clock: ClockId,
) -> RuntimeResult<u64> {
    let value = match clock {
        ClockId::Wall => runtime.runtime().time.wall_nanos(),
        ClockId::Monotonic => runtime.runtime().time.mono_nanos(),
        // NOTE #Incomplete: implement process cpu clock source
        ClockId::ProcessCpu => runtime.runtime().time.mono_nanos(),
        // NOTE #Incomplete: implement thread cpu clock source
        ClockId::ThreadCpu => runtime.runtime().time.mono_nanos(),
        // NOTE #Incomplete: implement boot clock source
        ClockId::Boot => runtime.runtime().time.mono_nanos(),
        // NOTE #Incomplete: implement monotonic raw clock source
        ClockId::MonotonicRaw => runtime.runtime().time.mono_nanos(),
    };

    Ok(value)
}

/// Return process cpu time in nanoseconds.
pub fn destack_time_process_cpu_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    // NOTE #Incomplete: implement process cpu timer source
    Ok(runtime.runtime().time.mono_nanos())
}

/// Return thread cpu time in nanoseconds.
pub fn destack_time_thread_cpu_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    // NOTE #Incomplete: implement thread cpu timer source
    Ok(runtime.runtime().time.mono_nanos())
}

/// Sleep for the given duration in nanoseconds.
pub fn destack_time_sleep_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    duration: u64,
) -> RuntimeResult<()> {
    runtime.runtime().time.sleep_nanos(duration);
    Ok(())
}

/// Sleep for the given duration on the selected clock.
pub fn destack_time_sleep_on_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    duration: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: honor sleep clock selection
    let _ = clock;
    runtime.runtime().time.sleep_nanos(duration);

    Ok(())
}

/// Sleep until the provided deadline in nanoseconds.
pub fn destack_time_sleep_until_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    deadline: u64,
) -> RuntimeResult<()> {
    runtime.runtime().time.sleep_until_nanos(deadline);
    Ok(())
}

/// Sleep until the given deadline on the selected clock.
pub fn destack_time_sleep_until_on_ns(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    deadline: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: honor sleep clock selection
    let _ = clock;
    runtime.runtime().time.sleep_until_nanos(deadline);

    Ok(())
}
