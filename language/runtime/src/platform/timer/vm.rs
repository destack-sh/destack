use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::timer::TimerOptionsVm;
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.timer.control.cancel.
pub(super) fn destack_timer_cancel(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.cancel is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.control.isActive.
pub(super) fn destack_timer_is_active(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<bool> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.isActive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.control.pause.
pub(super) fn destack_timer_pause(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.pause is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.control.remainingNs.
pub(super) fn destack_timer_remaining_ns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<u64> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.remainingNs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.control.reset.
pub(super) fn destack_timer_reset(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, delayns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.reset is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.control.resume.
pub(super) fn destack_timer_resume(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.resume is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.control.updateInterval.
pub(super) fn destack_timer_update_interval(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, periodns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.control.updateInterval is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.at.
pub(super) fn destack_timer_at(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    deadlinens: u64,
) -> RuntimeResult<resource::TimerHandle> {
    let _ = deadlinens;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.at is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.atWithOptions.
pub(super) fn destack_timer_at_with_options(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    deadlinens: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let _ = (deadlinens, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.atWithOptions is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.interval.
pub(super) fn destack_timer_interval(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    periodns: u64,
) -> RuntimeResult<resource::TimerHandle> {
    let _ = periodns;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.interval is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.intervalWithOptions.
pub(super) fn destack_timer_interval_with_options(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    periodns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let _ = (periodns, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.intervalWithOptions is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.once.
pub(super) fn destack_timer_once(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    delayns: u64,
) -> RuntimeResult<resource::TimerHandle> {
    let _ = delayns;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.once is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.timer.schedule.onceWithOptions.
pub(super) fn destack_timer_once_with_options(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    delayns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let _ = (delayns, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.timer.schedule.onceWithOptions is not available in the VM yet",
    ))
    .boxed())
}
