use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::time::{TimerClock, TimerOptions};
use crate::platform::{PlatformError, ResourceTable, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::scheduler::{Timer as LoopTimer, TimerDeadline};
use crate::runtime::time::{Clock, Nanos};
use destack_workspace::TimeMode;

/// Runtime state for one scheduled timer handle.
#[derive(Debug)]
struct TimerState {
    /// Clock domain used for deadline calculations.
    clock: TimerClock,
    /// Optional repeating interval.
    interval: Option<Nanos>,
    /// Next deadline in the selected clock domain.
    next_deadline: Nanos,
    /// Active marker for this timer handle.
    active: bool,
    /// Paused marker for this timer handle.
    paused: bool,
    /// Remaining duration captured during pause.
    paused_remaining: Nanos,
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
fn now_for_clock(context: &BindingCallContext, clock: TimerClock) -> Nanos {
    match clock {
        TimerClock::Wall => context.world().wall(),
        TimerClock::Monotonic => context.world().mono(),
    }
}

/// Resolve one timer clock domain into one current nanosecond timestamp.
fn now_for_timer_clock(clock: &Clock, time_mode: TimeMode, timer_clock: TimerClock) -> Nanos {
    match timer_clock {
        TimerClock::Wall => match time_mode {
            TimeMode::Host => clock.host_wall(),
            TimeMode::Virtual => clock.virtual_wall(),
        },
        TimerClock::Monotonic => match time_mode {
            TimeMode::Host => clock.host_mono(),
            TimeMode::Virtual => clock.virtual_mono(),
        },
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
    context: &BindingCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<Arc<Mutex<TimerState>>> {
    let state = context
        .worker()
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

/// Resolve one timer handle from one resource table.
fn timer_state_for_resources(
    resources: &ResourceTable,
    handle: resource::TimerHandle,
) -> Option<Arc<Mutex<TimerState>>> {
    resources
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
}

/// Update one timer state when one event loop timer fires.
pub(crate) fn on_event_loop_timer_fire(
    resources: &ResourceTable,
    clock: &Clock,
    time_mode: TimeMode,
    handle: resource::TimerHandle,
) -> RuntimeResult<bool> {
    // allow scheduler-managed timers that are not backed by one runtime resource entry
    let Some(state) = timer_state_for_resources(resources, handle) else {
        return Ok(true);
    };

    // update timer state according to one fired timer tick
    let mut state = state.lock();
    if !state.active || state.paused {
        return Ok(false);
    }

    // complete one-shot timers on the first fire
    if state.interval.is_none() {
        state.active = false;
        state.paused = false;
        state.paused_remaining = Nanos::new(0);
        return Ok(true);
    }

    // keep interval timers aligned to the next future deadline
    let interval = state.interval.unwrap_or(Nanos::new(0));
    if interval.get() == 0 {
        state.active = false;
        state.paused = false;
        state.paused_remaining = Nanos::new(0);
        return Ok(false);
    }

    state.next_deadline = state.next_deadline.saturating_add(interval);
    let now = now_for_timer_clock(clock, time_mode, state.clock);
    if state.next_deadline <= now {
        let elapsed = now.saturating_sub(state.next_deadline);
        let skipped_periods = elapsed.get() / interval.get() + 1;
        let skip_delta = interval.get().saturating_mul(skipped_periods);
        state.next_deadline = state.next_deadline.saturating_add(Nanos::new(skip_delta));
    }

    Ok(true)
}

/// Insert one timer state payload and return its handle.
fn insert_timer_state(context: &BindingCallContext, state: TimerState) -> resource::TimerHandle {
    let entry = ResourceEntry::new(ResourceKind::Timer)
        .with_label("timer.schedule")
        .with_payload(Arc::new(Mutex::new(state)));
    let resource_id =
        context
            .worker()
            .resources
            .insert(context.world(), entry, Some(context.engine()));
    resource::TimerHandle(resource_id)
}

/// Enqueue one active timer state into the event loop queue.
fn schedule_timer_state(
    context: &BindingCallContext,
    handle: resource::TimerHandle,
    state: &TimerState,
) -> RuntimeResult<()> {
    if !state.active || state.paused {
        return Ok(());
    }

    context.event_loop().schedule_timer(LoopTimer {
        handle: handle.0.into(),
        deadline: TimerDeadline {
            clock: state.clock,
            at: state.next_deadline,
        },
        interval: state.interval,
    })
}

/// Refresh one timer state against the current clock value.
fn refresh_timer_state(context: &BindingCallContext, state: &mut TimerState) {
    // skip inactive and paused timers
    if !state.active || state.paused {
        return;
    }

    // evaluate one-shot timer completion
    if state.interval.is_none() {
        let now = now_for_clock(context, state.clock);
        if now >= state.next_deadline {
            state.active = false;
        }
        return;
    }

    // roll repeating timers to the next future deadline
    let interval = state.interval.unwrap_or(Nanos::new(0));
    if interval.get() == 0 {
        state.active = false;
        return;
    }

    let now = now_for_clock(context, state.clock);
    if now < state.next_deadline {
        return;
    }

    let elapsed = now.saturating_sub(state.next_deadline);
    let periods = elapsed.get() / interval.get() + 1;
    let advance = interval.get().saturating_mul(periods);
    state.next_deadline = state.next_deadline.saturating_add(Nanos::new(advance));
}

/// Return remaining nanoseconds for one timer state snapshot.
fn remaining_nanos(context: &BindingCallContext, state: &TimerState) -> Nanos {
    if !state.active {
        return Nanos::new(0);
    }
    if state.paused {
        return state.paused_remaining;
    }

    let now = now_for_clock(context, state.clock);
    state.next_deadline.saturating_sub(now)
}

/// Create one timer state and schedule it.
fn create_timer(
    context: &BindingCallContext,
    options: TimerOptions,
    deadline_ns: u64,
    interval_ns: Option<u64>,
) -> RuntimeResult<resource::TimerHandle> {
    // validate timer options before creating state
    validate_timer_options(options)?;

    // insert one state payload into the resource table
    let state = TimerState {
        clock: options.clock,
        interval: interval_ns.map(Nanos::new),
        next_deadline: Nanos::new(deadline_ns),
        active: true,
        paused: false,
        paused_remaining: Nanos::new(0),
    };
    let handle = insert_timer_state(context, state);

    // schedule the timer in the runtime event loop
    let state = timer_state_for_handle(context, handle)?;
    let state = state.lock();
    if let Err(error) = schedule_timer_state(context, handle, &state) {
        let _ =
            context
                .worker()
                .resources
                .remove(context.world(), handle.0, Some(context.engine()));
        return Err(error);
    }

    Ok(handle)
}

/// Cancel a scheduled timer.
///
/// Remove one timer from the runtime event loop.
/// Cancellation is idempotent when supported by the runtime implementation.
pub(crate) unsafe fn destack_timer_cancel(
    context: &BindingCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    // remove one timer handle from the resource table
    let entry = context
        .worker()
        .resources
        .remove(context.world(), handle.0, Some(context.engine()))
        .ok_or_else(invalid_timer_handle_error)?;
    if entry.kind != ResourceKind::Timer {
        return Err(invalid_timer_handle_error());
    }

    // cancel one queued event loop timer entry
    context.event_loop().cancel_timer(handle.0)?;
    Ok(())
}

/// Return whether a timer is currently active.
///
/// Read active-state metadata for one timer handle.
/// Active state reflects runtime event loop ownership and cancellation state.
pub(crate) unsafe fn destack_timer_is_active(
    context: &BindingCallContext,
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
        context.event_loop().cancel_timer(handle.0)?;
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
    context: &BindingCallContext,
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
    state.paused_remaining = remaining_nanos(context, &state);
    state.paused = true;
    context.event_loop().cancel_timer(handle.0)?;
    Ok(())
}

/// Return remaining timer delay in nanoseconds.
///
/// Read remaining delay for one timer relative to its configured clock domain.
/// Remaining delay is zero when timer has fired or is inactive.
pub(crate) unsafe fn destack_timer_remaining_ns(
    context: &BindingCallContext,
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
        context.event_loop().cancel_timer(handle.0)?;
    }
    let remaining = remaining_nanos(context, &state);

    // write one remaining duration
    unsafe {
        *out = remaining.get();
    }
    Ok(())
}

/// Reset one timer with a new relative delay.
///
/// Replace one timer schedule with a new relative delay.
/// Reset semantics preserve timer identity and replay ordering.
pub(crate) unsafe fn destack_timer_reset(
    context: &BindingCallContext,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    // resolve one timer state payload
    let state = timer_state_for_handle(context, handle)?;
    let mut state = state.lock();

    // apply one new relative deadline and resume scheduling
    let now = now_for_clock(context, state.clock);
    state.next_deadline = now.saturating_add(Nanos::new(delayns));
    state.paused_remaining = Nanos::new(0);
    state.paused = false;
    state.active = true;
    context.event_loop().cancel_timer(handle.0)?;
    schedule_timer_state(context, handle, &state)
}

/// Resume a paused timer.
///
/// Reactivate one paused timer in the runtime event loop.
/// Resume timing semantics follow runtime timer policy.
pub(crate) unsafe fn destack_timer_resume(
    context: &BindingCallContext,
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
    state.next_deadline = now.saturating_add(state.paused_remaining);
    state.paused_remaining = Nanos::new(0);
    state.paused = false;
    schedule_timer_state(context, handle, &state)
}

/// Update one timer interval period.
///
/// Replace one interval timer period while preserving timer identity.
/// Update semantics are runtime-defined for already-expired intervals.
pub(crate) unsafe fn destack_timer_update_interval(
    context: &BindingCallContext,
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
    state.interval = Some(Nanos::new(periodns));

    // keep paused and inactive timers unscheduled
    if !state.active || state.paused {
        return Ok(());
    }

    // reschedule the timer with the updated interval
    refresh_timer_state(context, &mut state);
    context.event_loop().cancel_timer(handle.0)?;
    schedule_timer_state(context, handle, &state)
}

/// Schedule a timer for an absolute wall-clock deadline.
///
/// Register one timer that fires at a specific deadline in nanoseconds with explicit timer options.
/// Deadline interpretation follows runtime wall-clock and monotonic policy.
pub(crate) unsafe fn destack_timer_at(
    context: &BindingCallContext,
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
    context: &BindingCallContext,
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
    let deadline = now.saturating_add(Nanos::new(periodns));
    let handle = create_timer(context, options, deadline.get(), Some(periodns))?;

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
    context: &BindingCallContext,
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
    let deadline = now.saturating_add(Nanos::new(delayns));
    let handle = create_timer(context, options, deadline.get(), None)?;

    // write one output handle
    unsafe {
        *out = handle;
    }
    Ok(())
}
