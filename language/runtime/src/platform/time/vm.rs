use destack_vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::call_out;
use crate::platform::resource;
use crate::platform::time::{
    ClockId, ClockMetadataVm, SleepClock, TimerOptions, TimerOptionsVm, native as runtime_native,
};
use crate::runtime::{self, BindingCallContext};

/// Query one selected clock metadata.
pub(crate) fn destack_time_clock_metadata(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    clock: ClockId,
) -> RuntimeResult<ClockMetadataVm> {
    call_out(|out| unsafe { runtime::time::host::host_clock_metadata(binding, out, clock) })
}

/// Return monotonic time in nanoseconds.
pub(crate) fn destack_time_mono_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { runtime::time::host::host_mono_nanos(binding, out) })
}

/// Read one selected clock in nanoseconds.
pub(crate) fn destack_time_now_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    clock: ClockId,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { runtime::time::host::host_now_nanos(binding, out, clock) })
}

/// Return process CPU time in nanoseconds.
pub(crate) fn destack_time_process_cpu_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { runtime::time::host::host_process_cpu_nanos(binding, out) })
}

/// Return current thread CPU time in nanoseconds.
pub(crate) fn destack_time_thread_cpu_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { runtime::time::host::host_thread_cpu_nanos(binding, out) })
}

/// Return wall clock time in nanoseconds since the runtime epoch.
pub(crate) fn destack_time_wall_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { runtime::time::host::host_wall_nanos(binding, out) })
}

/// Sleep for at least the given duration in nanoseconds.
pub(crate) fn destack_time_sleep_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    duration: u64,
) -> RuntimeResult<()> {
    unsafe { runtime::time::host::host_sleep_nanos(binding, duration) }
}

/// Sleep for at least the given duration on one clock domain.
pub(crate) fn destack_time_sleep_on_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    duration: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    unsafe { runtime::time::host::host_sleep_on_nanos(binding, duration, clock) }
}

/// Sleep until the given wall-clock deadline in nanoseconds.
pub(crate) fn destack_time_sleep_until_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    deadline: u64,
) -> RuntimeResult<()> {
    unsafe { runtime::time::host::host_sleep_until_nanos(binding, deadline) }
}

/// Sleep until one deadline on one clock domain.
pub(crate) fn destack_time_sleep_until_on_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    deadline: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    unsafe { runtime::time::host::host_sleep_until_on_nanos(binding, deadline, clock) }
}

/// Schedule a timer for an absolute deadline.
pub(crate) fn destack_time_timer_at(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    deadlinens: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let options = TimerOptions {
        clock: options.clock,
        flags: options.flags,
    };

    call_out(|out| unsafe {
        runtime_native::destack_time_timer_at(binding, out, deadlinens, options)
    })
}

/// Cancel one scheduled timer.
pub(crate) fn destack_time_timer_cancel(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_cancel(binding, handle) }
}

/// Schedule a repeating timer.
pub(crate) fn destack_time_timer_interval(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    periodns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let options = TimerOptions {
        clock: options.clock,
        flags: options.flags,
    };

    call_out(|out| unsafe {
        runtime_native::destack_time_timer_interval(binding, out, periodns, options)
    })
}

/// Return whether one timer is currently active.
pub(crate) fn destack_time_timer_is_active(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { runtime_native::destack_time_timer_is_active(binding, out, handle) })
}

/// Schedule a one-shot timer.
pub(crate) fn destack_time_timer_once(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    delayns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let options = TimerOptions {
        clock: options.clock,
        flags: options.flags,
    };

    call_out(|out| unsafe {
        runtime_native::destack_time_timer_once(binding, out, delayns, options)
    })
}

/// Pause one running timer.
pub(crate) fn destack_time_timer_pause(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_pause(binding, handle) }
}

/// Return remaining timer delay in nanoseconds.
pub(crate) fn destack_time_timer_remaining_ns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { runtime_native::destack_time_timer_remaining_ns(binding, out, handle) })
}

/// Reset one timer with a new relative delay.
pub(crate) fn destack_time_timer_reset(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_reset(binding, handle, delayns) }
}

/// Resume one paused timer.
pub(crate) fn destack_time_timer_resume(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_resume(binding, handle) }
}

/// Update one timer interval period.
pub(crate) fn destack_time_timer_update_interval(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_update_interval(binding, handle, periodns) }
}
