use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{TimerFdHandle, TimerHandle};
use crate::platform::timer::{
    TimerFdClock, TimerFdFlags, TimerFdSetFlags, TimerFdSpecVm, TimerOptionsVm,
};
use crate::platform::{PlatformError, ResourceEntry, ResourceKind};
use crate::runtime::RuntimeCallContext;
use crate::scheduler::Timer;
use destack_vm as vm;

/// Return a not supported vm binding error.
fn vm_not_supported(binding_name: &'static str) -> RuntimeResult<()> {
    let message = format!("{binding_name} is not available in the VM yet");
    Err(RuntimeError::from(PlatformError::not_supported(message)).boxed())
}

/// Schedule a timer and return its handle.
fn schedule_timer(
    runtime: &RuntimeCallContext,
    fire_at_nanos: u64,
    interval_nanos: Option<u64>,
) -> RuntimeResult<TimerHandle> {
    // allocate a timer resource id
    let resource_id = runtime
        .runtime()
        .resources
        .insert(ResourceEntry::new(ResourceKind::Timer));
    let handle = TimerHandle(resource_id);

    // schedule the timer in the runtime event loop
    let timer = Timer {
        handle: resource_id,
        fire_at_nanos,
        interval_nanos,
    };
    runtime.scheduler().schedule_timer(timer)?;

    Ok(handle)
}

/// Schedule a one shot timer and return its handle.
pub fn destack_timer_once(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    delayns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<TimerHandle> {
    // NOTE #Incomplete: honor timer options in vm mode
    let _ = options;

    // compute the absolute deadline from wall time
    let now = runtime.runtime().time.wall_nanos();
    let fire_at_nanos = now.saturating_add(delayns);

    schedule_timer(runtime, fire_at_nanos, None)
}

/// Schedule a repeating timer and return its handle.
pub fn destack_timer_interval(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    periodns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<TimerHandle> {
    // NOTE #Incomplete: honor timer options in vm mode
    let _ = options;

    // compute the first deadline from wall time
    let now = runtime.runtime().time.wall_nanos();
    let fire_at_nanos = now.saturating_add(periodns);

    schedule_timer(runtime, fire_at_nanos, Some(periodns))
}

/// Cancel a scheduled timer.
pub fn destack_timer_cancel(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    runtime.scheduler().cancel_timer(handle.0)
}

/// Schedule a timer for an absolute wall-clock deadline.
pub fn destack_timer_at(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    deadlinens: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<TimerHandle> {
    // NOTE #Incomplete: honor timer options in vm mode
    let _ = options;

    schedule_timer(runtime, deadlinens, None)
}

/// Schedule a timer for an absolute deadline with options.
pub fn destack_timer_at_with_options(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    deadlinens: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<TimerHandle> {
    destack_timer_at(runtime, context, deadlinens, options)
}

/// Schedule a one shot timer with options.
pub fn destack_timer_once_with_options(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    delayns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<TimerHandle> {
    destack_timer_once(runtime, context, delayns, options)
}

/// Schedule an interval timer with options.
pub fn destack_timer_interval_with_options(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    periodns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<TimerHandle> {
    destack_timer_interval(runtime, context, periodns, options)
}

/// Report whether a timer is active.
pub fn destack_timer_is_active(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerHandle,
) -> RuntimeResult<bool> {
    // NOTE #Incomplete: implement vm timer activity query
    let _ = handle;
    vm_not_supported("destack.timer.isActive")?;

    unreachable!()
}

/// Pause a timer.
pub fn destack_timer_pause(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm timer pause
    let _ = handle;
    vm_not_supported("destack.timer.pause")
}

/// Resume a timer.
pub fn destack_timer_resume(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm timer resume
    let _ = handle;
    vm_not_supported("destack.timer.resume")
}

/// Read remaining nanoseconds for a timer.
pub fn destack_timer_remaining_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerHandle,
) -> RuntimeResult<u64> {
    // NOTE #Incomplete: implement vm remaining-time query
    let _ = handle;
    vm_not_supported("destack.timer.remainingNs")?;

    unreachable!()
}

/// Reset a timer delay.
pub fn destack_timer_reset(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm timer reset
    let _ = (handle, delayns);
    vm_not_supported("destack.timer.reset")
}

/// Update an interval timer period.
pub fn destack_timer_update_interval(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm interval update
    let _ = (handle, periodns);
    vm_not_supported("destack.timer.updateInterval")
}

/// Close a timerfd style descriptor.
pub fn destack_timer_timer_fd_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    vm_not_supported("destack.timer.timerFdClose")
}

/// Read a timerfd specification.
pub fn destack_timer_timer_fd_get(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerFdHandle,
) -> RuntimeResult<TimerFdSpecVm> {
    let _ = handle;
    vm_not_supported("destack.timer.timerFdGet")?;
    unreachable!()
}

/// Open a timerfd style descriptor.
pub fn destack_timer_timer_fd_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    clock: TimerFdClock,
    flags: TimerFdFlags,
) -> RuntimeResult<TimerFdHandle> {
    let _ = (clock, flags);
    vm_not_supported("destack.timer.timerFdOpen")?;
    unreachable!()
}

/// Read expirations from a timerfd descriptor.
pub fn destack_timer_timer_fd_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerFdHandle,
) -> RuntimeResult<u64> {
    let _ = handle;
    vm_not_supported("destack.timer.timerFdRead")?;
    unreachable!()
}

/// Update the timerfd specification.
pub fn destack_timer_timer_fd_set(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: TimerFdHandle,
    spec: TimerFdSpecVm,
    flags: TimerFdSetFlags,
) -> RuntimeResult<()> {
    let _ = (handle, spec, flags);
    vm_not_supported("destack.timer.timerFdSet")
}
