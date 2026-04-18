#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::mem::MaybeUninit;
use std::os::fd::RawFd;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::io::{TimerFdClock, TimerFdFlags, TimerFdSetFlags, TimerFdSpec};
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::resource::ResourceEntry;
use crate::platform::resource::ResourceKind;
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Number of nanoseconds in one second.
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Return one invalid-handle error for timerfd operations.
fn invalid_timerfd_handle_error() -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "handle",
        "unknown timerfd handle",
    ))
    .boxed()
}

/// Return one unsupported-flags error.
fn unsupported_flags_error(field: &str, flags: u32) -> Box<RuntimeError> {
    core_platform::unsupported_flags(field, flags)
}

/// Return one ioWouldBlock error from one unix errno value.
fn io_would_block_error(operation: &str, errno: i32) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        Some(errno),
        Some(operation.to_string()),
        None,
        format!("{operation} would block"),
    ))
    .boxed()
}

/// Convert one nanosecond timeout into one libc timespec.
fn nanos_to_timespec(value: u64, field: &str) -> RuntimeResult<libc::timespec> {
    // split the nanosecond value into seconds and subsecond nanos
    let seconds = value / NANOS_PER_SECOND;
    let nanos = value % NANOS_PER_SECOND;

    // validate the host seconds range
    let seconds = libc::time_t::try_from(seconds).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "value exceeds host timespec range",
        ))
        .boxed()
    })?;

    Ok(libc::timespec {
        tv_sec: seconds,
        tv_nsec: nanos as libc::c_long,
    })
}

/// Convert one libc timespec into one nanosecond value.
fn timespec_to_nanos(value: libc::timespec, field: &str) -> RuntimeResult<u64> {
    // reject one negative host timespec value
    if value.tv_sec < 0 || value.tv_nsec < 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "host returned one negative timespec",
        ))
        .boxed());
    }

    // convert one timespec into nanoseconds
    let seconds = u64::try_from(value.tv_sec).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "seconds do not fit in uint64",
        ))
        .boxed()
    })?;
    let nanos = u64::try_from(value.tv_nsec).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "nanoseconds do not fit in uint64",
        ))
        .boxed()
    })?;
    let seconds_nanos = seconds.checked_mul(NANOS_PER_SECOND).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "seconds overflow nanosecond conversion",
        ))
        .boxed()
    })?;

    Ok(seconds_nanos.saturating_add(nanos))
}

/// Resolve one timerfd handle into one unix file descriptor.
fn timerfd_fd(
    binding: &BindingCallContext,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<RawFd> {
    let fd = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::TimerFd {
                return None;
            }
            entry.fd()
        })
        .flatten()
        .ok_or_else(invalid_timerfd_handle_error)?;

    Ok(fd)
}

/// Close one timerfd descriptor and release one table entry.
fn close_timerfd(
    binding: &BindingCallContext,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    // remove one timerfd entry from the resource table
    let entry = binding
        .worker()
        .resources
        .remove(&binding.world(), handle.0, Some(binding.engine()))
        .ok_or_else(invalid_timerfd_handle_error)?;
    if entry.kind != ResourceKind::TimerFd {
        return Err(invalid_timerfd_handle_error());
    }

    let fd = entry.fd().ok_or_else(invalid_timerfd_handle_error)?;

    // close the host timerfd descriptor
    let rc = unsafe { libc::close(fd) };
    if rc != 0 {
        return Err(core_platform::io_error("close", None));
    }

    Ok(())
}

/// Return whether timerfd is available on this unix target.
#[cfg(any(target_os = "linux", target_os = "android"))]
const fn timerfd_available() -> bool {
    true
}

/// Return whether timerfd is available on this unix target.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
const fn timerfd_available() -> bool {
    false
}

/// Map one timerfd clock enum into one linux clock identifier.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn timerfd_clock_id(clock: TimerFdClock) -> RuntimeResult<libc::clockid_t> {
    match clock {
        TimerFdClock::Realtime => Ok(libc::CLOCK_REALTIME),
        TimerFdClock::Monotonic => Ok(libc::CLOCK_MONOTONIC),
        TimerFdClock::Boottime => Ok(libc::CLOCK_BOOTTIME),
    }
}

/// Validate one timerfd open flag mask.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn validate_open_flags(flags: TimerFdFlags) -> RuntimeResult<i32> {
    let allowed = (libc::TFD_CLOEXEC | libc::TFD_NONBLOCK) as u32;
    if flags.0 & !allowed != 0 {
        return Err(unsupported_flags_error("flags", flags.0));
    }

    Ok(flags.0 as i32)
}

/// Validate one timerfd set flag mask.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn validate_set_flags(flags: TimerFdSetFlags) -> RuntimeResult<i32> {
    let allowed = (libc::TFD_TIMER_ABSTIME | libc::TFD_TIMER_CANCEL_ON_SET) as u32;
    if flags.0 & !allowed != 0 {
        return Err(unsupported_flags_error("flags", flags.0));
    }

    Ok(flags.0 as i32)
}

/// Close one timerfd descriptor.
///
/// Close one descriptor and release host timer queue resources.
/// Pending expirations are discarded according to host close semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_close(
    binding: &BindingCallContext,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    // reject timerfd operations on unsupported unix targets
    if !timerfd_available() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.close")).boxed(),
        );
    }

    close_timerfd(binding, handle)
}

/// Read the active timerfd schedule.
///
/// Return one normalized schedule snapshot for the descriptor.
/// Returned values are measured in nanoseconds using host timerfd conversion rules.
///
/// # Platform
/// Unix and Windows.
/// Uses timerfd_gettime(2) on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_get(
    binding: &BindingCallContext,
    out: *mut TimerFdSpec,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject timerfd operations on unsupported unix targets
    if !timerfd_available() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.get")).boxed(),
        );
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // resolve one host timerfd descriptor
        let fd = timerfd_fd(binding, handle)?;

        // read one host timerfd schedule
        let mut host_spec = MaybeUninit::<libc::itimerspec>::zeroed();
        let rc = unsafe { libc::timerfd_gettime(fd, host_spec.as_mut_ptr()) };
        if rc != 0 {
            return Err(core_platform::io_error("timerfd_gettime", None));
        }
        let host_spec = unsafe { host_spec.assume_init() };

        // convert the host schedule payload into binding form
        let spec = TimerFdSpec {
            initial_ns: timespec_to_nanos(host_spec.it_value, "initialNs")?,
            interval_ns: timespec_to_nanos(host_spec.it_interval, "intervalNs")?,
        };
        unsafe {
            *out = spec;
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.get")).boxed())
    }
}

/// Open one timerfd style descriptor.
///
/// Create one descriptor-backed timer queue in the requested clock domain.
/// Timerfd behavior and descriptor flags follow host kernel semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses timerfd_create(2) on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_open(
    binding: &BindingCallContext,
    out: *mut resource::TimerFdHandle,
    clock: TimerFdClock,
    flags: TimerFdFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject timerfd operations on unsupported unix targets
    if !timerfd_available() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.open")).boxed(),
        );
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // decode one host timerfd clock and open flags
        let clock_id = timerfd_clock_id(clock)?;
        let flags = validate_open_flags(flags)?;

        // open one timerfd descriptor
        let fd = unsafe { libc::timerfd_create(clock_id, flags) };
        if fd < 0 {
            return Err(core_platform::io_error("timerfd_create", None));
        }

        // register one timerfd resource entry
        let entry = ResourceEntry::new(ResourceKind::TimerFd)
            .with_label("io.timerfd")
            .with_fd(fd);
        let resource_id =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));
        let handle = resource::TimerFdHandle(resource_id);

        // write one output handle
        unsafe {
            *out = handle;
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (binding, clock, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.open")).boxed())
    }
}

/// Read one timerfd expiration counter.
///
/// Consume one pending expiration counter value from the descriptor.
/// Counter semantics follow host timerfd read behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses read(2) on timerfd descriptors on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject timerfd operations on unsupported unix targets
    if !timerfd_available() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.read")).boxed(),
        );
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // resolve one host timerfd descriptor
        let fd = timerfd_fd(binding, handle)?;

        // read one expiration counter from the descriptor
        let mut expirations = 0_u64;
        let rc = unsafe {
            libc::read(
                fd,
                (&mut expirations as *mut u64).cast::<libc::c_void>(),
                std::mem::size_of::<u64>(),
            )
        };
        if rc < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
                return Err(io_would_block_error("read", errno));
            }

            return Err(core_platform::io_error("read", None));
        }
        if rc as usize != std::mem::size_of::<u64>() {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoInvalidData),
                None,
                None,
                Some("read".to_string()),
                None,
                "short timerfd read",
            ))
            .boxed());
        }

        // write one expiration count
        unsafe {
            *out = expirations;
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.read")).boxed())
    }
}

/// Update one timerfd schedule.
///
/// Replace the timer schedule with one initial deadline and one interval period.
/// Absolute or relative interpretation is controlled by the provided set flags.
///
/// # Platform
/// Unix and Windows.
/// Uses timerfd_settime(2) on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_set(
    binding: &BindingCallContext,
    handle: resource::TimerFdHandle,
    spec: TimerFdSpec,
    flags: TimerFdSetFlags,
) -> RuntimeResult<()> {
    // reject timerfd operations on unsupported unix targets
    if !timerfd_available() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.set")).boxed(),
        );
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // resolve one host timerfd descriptor and set flags
        let fd = timerfd_fd(binding, handle)?;
        let flags = validate_set_flags(flags)?;

        // build one host timerfd schedule payload
        let host_spec = libc::itimerspec {
            it_interval: nanos_to_timespec(spec.interval_ns, "intervalNs")?,
            it_value: nanos_to_timespec(spec.initial_ns, "initialNs")?,
        };

        // update one host timerfd schedule
        let rc = unsafe { libc::timerfd_settime(fd, flags, &host_spec, std::ptr::null_mut()) };
        if rc != 0 {
            return Err(core_platform::io_error("timerfd_settime", None));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (binding, handle, spec, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.set")).boxed())
    }
}
