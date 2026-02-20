use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::time::{TimerClock, TimerOptions};
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;
use crate::runtime::scheduler::Timer as SchedulerTimer;

/// Runtime state for one scheduled timer handle.
#[derive(Debug)]
struct TimerState {
    /// Clock domain used for deadline calculations.
    clock: TimerClock,
    /// Optional repeating interval in nanoseconds.
    interval_ns: Option<u64>,
    /// Next deadline in the selected clock domain.
    next_deadline_ns: u64,
    /// Active marker for this timer handle.
    active: bool,
    /// Paused marker for this timer handle.
    paused: bool,
    /// Remaining duration captured during pause.
    paused_remaining_ns: u64,
}

/// Return one invalid timer-handle error.
fn invalid_timer_handle_error() -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "handle",
        "unknown timer handle",
    ))
    .boxed()
}

/// Return one unsupported-flags error.
fn unsupported_flags_error(flags: u32) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "options.flags",
        format!("unsupported flag bits: 0x{flags:x}"),
    ))
    .boxed()
}

/// Return one invalid-period error.
fn invalid_period_error(field: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        field,
        "period must be greater than zero",
    ))
    .boxed()
}

/// Resolve one clock domain into one current nanosecond timestamp.
fn now_for_clock(context: &RuntimeCallContext, clock: TimerClock) -> u64 {
    match clock {
        TimerClock::Wall => context.runtime().time.wall_nanos(),
        TimerClock::Monotonic => context.runtime().time.mono_nanos(),
    }
}

/// Convert one clock-domain deadline into one scheduler wall-clock deadline.
fn scheduler_deadline(context: &RuntimeCallContext, clock: TimerClock, deadline_ns: u64) -> u64 {
    match clock {
        TimerClock::Wall => deadline_ns,
        TimerClock::Monotonic => {
            let wall_now = context.runtime().time.wall_nanos();
            let mono_now = context.runtime().time.mono_nanos();
            let delta = deadline_ns.saturating_sub(mono_now);
            wall_now.saturating_add(delta)
        }
    }
}

/// Validate one timer options payload.
fn validate_timer_options(options: TimerOptions) -> RuntimeResult<()> {
    if options.flags.0 != 0 {
        return Err(unsupported_flags_error(options.flags.0));
    }

    Ok(())
}

/// Resolve one timer handle into one mutable state payload.
fn timer_state_for_handle(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<Arc<Mutex<TimerState>>> {
    let state = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Timer {
                return None;
            }
            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<Mutex<TimerState>>>())
                .map(Arc::clone)
        })
        .flatten()
        .ok_or_else(invalid_timer_handle_error)?;

    Ok(state)
}

/// Insert one timer state payload and return its handle.
fn insert_timer_state(context: &RuntimeCallContext, state: TimerState) -> resource::TimerHandle {
    let entry = ResourceEntry::new(ResourceKind::Timer)
        .with_label("timer.schedule")
        .with_payload(Arc::new(Mutex::new(state)));
    let resource_id = context.runtime().resources.insert(entry);
    resource::TimerHandle(resource_id)
}

/// Enqueue one active timer state into the scheduler queue.
fn schedule_timer_state(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
    state: &TimerState,
) -> RuntimeResult<()> {
    if !state.active || state.paused {
        return Ok(());
    }

    let fire_at_nanos = scheduler_deadline(context, state.clock, state.next_deadline_ns);
    context.scheduler().schedule_timer(SchedulerTimer {
        handle: handle.0,
        fire_at_nanos,
        interval_nanos: state.interval_ns,
    })
}

/// Refresh one timer state against the current clock value.
fn refresh_timer_state(context: &RuntimeCallContext, state: &mut TimerState) {
    // skip inactive and paused timers
    if !state.active || state.paused {
        return;
    }

    // evaluate one-shot timer completion
    if state.interval_ns.is_none() {
        let now = now_for_clock(context, state.clock);
        if now >= state.next_deadline_ns {
            state.active = false;
        }
        return;
    }

    // roll repeating timers to the next future deadline
    let interval = state.interval_ns.unwrap_or(0);
    if interval == 0 {
        state.active = false;
        return;
    }

    let now = now_for_clock(context, state.clock);
    if now < state.next_deadline_ns {
        return;
    }

    let elapsed = now.saturating_sub(state.next_deadline_ns);
    let periods = elapsed / interval + 1;
    let advance = interval.saturating_mul(periods);
    state.next_deadline_ns = state.next_deadline_ns.saturating_add(advance);
}

/// Return remaining nanoseconds for one timer state snapshot.
fn remaining_nanos(context: &RuntimeCallContext, state: &TimerState) -> u64 {
    if !state.active {
        return 0;
    }
    if state.paused {
        return state.paused_remaining_ns;
    }

    let now = now_for_clock(context, state.clock);
    state.next_deadline_ns.saturating_sub(now)
}

/// Create one timer state and schedule it.
fn create_timer(
    context: &RuntimeCallContext,
    options: TimerOptions,
    deadline_ns: u64,
    interval_ns: Option<u64>,
) -> RuntimeResult<resource::TimerHandle> {
    // validate timer options before creating state
    validate_timer_options(options)?;

    // insert one state payload into the resource table
    let state = TimerState {
        clock: options.clock,
        interval_ns,
        next_deadline_ns: deadline_ns,
        active: true,
        paused: false,
        paused_remaining_ns: 0,
    };
    let handle = insert_timer_state(context, state);

    // schedule the timer in the runtime scheduler
    let state = timer_state_for_handle(context, handle)?;
    let state = state.lock();
    if let Err(error) = schedule_timer_state(context, handle, &state) {
        let _ = context.runtime().resources.remove(handle.0);
        return Err(error);
    }

    Ok(handle)
}

/// Cancel a scheduled timer.
///
/// Remove one timer from the runtime scheduler.
/// Cancellation is idempotent when supported by the runtime implementation.
pub(crate) unsafe fn destack_timer_cancel(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    // remove one timer handle from the resource table
    let entry = context
        .runtime()
        .resources
        .remove(handle.0)
        .ok_or_else(invalid_timer_handle_error)?;
    if entry.kind != ResourceKind::Timer {
        return Err(invalid_timer_handle_error());
    }

    // cancel one queued scheduler timer entry
    context.scheduler().cancel_timer(handle.0)?;
    Ok(())
}

/// Return whether a timer is currently active.
///
/// Read active-state metadata for one timer handle.
/// Active state reflects runtime scheduler ownership and cancellation state.
pub(crate) unsafe fn destack_timer_is_active(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // refresh one timer state before reporting active status
    let state = timer_state_for_handle(context, handle)?;
    let mut state = state.lock();
    refresh_timer_state(context, &mut state);
    if !state.active {
        context.scheduler().cancel_timer(handle.0)?;
    }

    // write one active marker
    unsafe {
        *out = state.active;
    }
    Ok(())
}

/// Pause a running timer.
///
/// Suspend one timer without discarding its scheduling state.
/// Resume behavior and retained delay follow the active runtime timer policy.
pub(crate) unsafe fn destack_timer_pause(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    // refresh one timer state before pausing
    let state = timer_state_for_handle(context, handle)?;
    let mut state = state.lock();
    refresh_timer_state(context, &mut state);
    if !state.active || state.paused {
        return Ok(());
    }

    // capture one remaining duration and pause scheduling
    state.paused_remaining_ns = remaining_nanos(context, &state);
    state.paused = true;
    context.scheduler().cancel_timer(handle.0)?;
    Ok(())
}

/// Return remaining timer delay in nanoseconds.
///
/// Read remaining delay for one timer relative to its configured clock domain.
/// Remaining delay is zero when timer has fired or is inactive.
pub(crate) unsafe fn destack_timer_remaining_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // refresh one timer state before reporting remaining time
    let state = timer_state_for_handle(context, handle)?;
    let mut state = state.lock();
    refresh_timer_state(context, &mut state);
    if !state.active {
        context.scheduler().cancel_timer(handle.0)?;
    }
    let remaining = remaining_nanos(context, &state);

    // write one remaining duration
    unsafe {
        *out = remaining;
    }
    Ok(())
}

/// Reset one timer with a new relative delay.
///
/// Replace one timer schedule with a new relative delay.
/// Reset semantics preserve timer identity and replay ordering.
pub(crate) unsafe fn destack_timer_reset(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    // resolve one timer state payload
    let state = timer_state_for_handle(context, handle)?;
    let mut state = state.lock();

    // apply one new relative deadline and resume scheduling
    let now = now_for_clock(context, state.clock);
    state.next_deadline_ns = now.saturating_add(delayns);
    state.paused_remaining_ns = 0;
    state.paused = false;
    state.active = true;
    context.scheduler().cancel_timer(handle.0)?;
    schedule_timer_state(context, handle, &state)
}

/// Resume a paused timer.
///
/// Reactivate one paused timer in the runtime scheduler.
/// Resume timing semantics follow runtime timer policy.
pub(crate) unsafe fn destack_timer_resume(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    // resolve one timer state payload
    let state = timer_state_for_handle(context, handle)?;
    let mut state = state.lock();
    if !state.active || !state.paused {
        return Ok(());
    }

    // restore one deadline from paused remaining duration
    let now = now_for_clock(context, state.clock);
    state.next_deadline_ns = now.saturating_add(state.paused_remaining_ns);
    state.paused_remaining_ns = 0;
    state.paused = false;
    schedule_timer_state(context, handle, &state)
}

/// Update one timer interval period.
///
/// Replace one interval timer period while preserving timer identity.
/// Update semantics are runtime-defined for already-expired intervals.
pub(crate) unsafe fn destack_timer_update_interval(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    // validate the interval period
    if periodns == 0 {
        return Err(invalid_period_error("periodNs"));
    }

    // resolve one timer state payload
    let state = timer_state_for_handle(context, handle)?;
    let mut state = state.lock();
    state.interval_ns = Some(periodns);

    // keep paused and inactive timers unscheduled
    if !state.active || state.paused {
        return Ok(());
    }

    // reschedule the timer with the updated interval
    refresh_timer_state(context, &mut state);
    context.scheduler().cancel_timer(handle.0)?;
    schedule_timer_state(context, handle, &state)
}

/// Schedule a timer for an absolute wall-clock deadline.
///
/// Register one timer that fires at a specific deadline in nanoseconds with explicit timer options.
/// Deadline interpretation follows runtime wall-clock and monotonic policy.
pub(crate) unsafe fn destack_timer_at(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    deadlinens: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // create and schedule one timer handle
    let handle = create_timer(context, options, deadlinens, None)?;

    // write one output handle
    unsafe {
        *out = handle;
    }
    Ok(())
}

/// Schedule a repeating timer.
///
/// Register one timer that fires repeatedly at a fixed period with explicit timer options.
/// Drift and catch-up behavior follow runtime timer policy.
pub(crate) unsafe fn destack_timer_interval(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    periodns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    // validate the output pointer and period
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    if periodns == 0 {
        return Err(invalid_period_error("periodNs"));
    }

    // create and schedule one repeating timer handle
    let now = now_for_clock(context, options.clock);
    let deadline = now.saturating_add(periodns);
    let handle = create_timer(context, options, deadline, Some(periodns))?;

    // write one output handle
    unsafe {
        *out = handle;
    }
    Ok(())
}

/// Schedule a one-shot timer.
///
/// Register one timer that fires once after a relative delay with explicit timer options.
/// The handle remains valid until explicit cancel or one-shot completion.
pub(crate) unsafe fn destack_timer_once(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    delayns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // create and schedule one one-shot timer handle
    let now = now_for_clock(context, options.clock);
    let deadline = now.saturating_add(delayns);
    let handle = create_timer(context, options, deadline, None)?;

    // write one output handle
    unsafe {
        *out = handle;
    }
    Ok(())
}
