#[cfg(target_os = "linux")]
use std::sync::OnceLock;
use std::time::Duration;
#[cfg(target_vendor = "apple")]
use std::time::Instant;
#[cfg(not(target_vendor = "apple"))]
use std::time::{SystemTime, UNIX_EPOCH};

use crate::platform::core as core_platform;

/// Poll interval for one Apple semaphore timed-wait fallback slice.
#[cfg(target_vendor = "apple")]
const APPLE_SEMAPHORE_WAIT_SLICE: Duration = Duration::from_millis(1);
/// Symbol name for the optional Linux monotonic semaphore wait entrypoint.
#[cfg(target_os = "linux")]
const LINUX_SEM_CLOCKWAIT_SYMBOL: &[u8] = b"sem_clockwait\0";

/// Linux monotonic semaphore timed-wait function pointer.
#[cfg(target_os = "linux")]
type LinuxSemaphoreClockwait =
    unsafe extern "C" fn(*mut libc::sem_t, libc::clockid_t, *const libc::timespec) -> libc::c_int;

/// One timed semaphore wait result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnixSemaphoreWaitStatus {
    /// One semaphore permit was acquired.
    Acquired,
    /// The requested timeout elapsed before one permit was acquired.
    TimedOut,
}

/// Convert one relative duration into one absolute realtime host timespec.
#[cfg(not(target_vendor = "apple"))]
fn absolute_realtime_timespec(deadline: SystemTime) -> Result<libc::timespec, i32> {
    let duration = deadline
        .duration_since(UNIX_EPOCH)
        .map_err(|_| libc::EINVAL)?;
    let seconds = i64::try_from(duration.as_secs()).map_err(|_| libc::EINVAL)?;

    Ok(libc::timespec {
        tv_sec: seconds as libc::time_t,
        tv_nsec: duration.subsec_nanos() as libc::c_long,
    })
}

/// Convert one relative duration into one absolute monotonic host timespec.
#[cfg(target_os = "linux")]
fn absolute_monotonic_timespec(timeout: Duration) -> Result<libc::timespec, i32> {
    // read one monotonic clock value
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now) };
    if rc != 0 {
        return Err(core_platform::get_errno());
    }

    // add the relative timeout to the current monotonic clock value
    let timeout_seconds = i64::try_from(timeout.as_secs()).map_err(|_| libc::EINVAL)?;
    let timeout_nanoseconds = i64::from(timeout.subsec_nanos());

    let mut deadline_seconds = now.tv_sec.saturating_add(timeout_seconds as libc::time_t);
    let mut deadline_nanoseconds = now
        .tv_nsec
        .saturating_add(timeout_nanoseconds as libc::c_long);

    if deadline_nanoseconds >= 1_000_000_000 {
        deadline_seconds = deadline_seconds.saturating_add(1);
        deadline_nanoseconds -= 1_000_000_000;
    }

    Ok(libc::timespec {
        tv_sec: deadline_seconds,
        tv_nsec: deadline_nanoseconds,
    })
}

/// Resolve the optional Linux monotonic semaphore wait entrypoint.
#[cfg(target_os = "linux")]
fn linux_sem_clockwait() -> Option<LinuxSemaphoreClockwait> {
    static SEM_CLOCKWAIT: OnceLock<Option<LinuxSemaphoreClockwait>> = OnceLock::new();

    *SEM_CLOCKWAIT.get_or_init(resolve_linux_sem_clockwait)
}

/// Resolve the optional Linux `sem_clockwait` symbol from the current process.
#[cfg(target_os = "linux")]
fn resolve_linux_sem_clockwait() -> Option<LinuxSemaphoreClockwait> {
    // clear any stale dynamic-loader state before probing the symbol
    unsafe {
        libc::dlerror();
    }

    // resolve the monotonic semaphore wait helper when libc exports it
    let symbol = unsafe {
        libc::dlsym(
            libc::RTLD_DEFAULT,
            LINUX_SEM_CLOCKWAIT_SYMBOL.as_ptr().cast::<libc::c_char>(),
        )
    };
    let error = unsafe { libc::dlerror() };
    if !error.is_null() || symbol.is_null() {
        return None;
    }

    // cast the raw symbol pointer into the typed function pointer
    union SymbolCast {
        pointer: *mut libc::c_void,
        symbol: LinuxSemaphoreClockwait,
    }

    Some(unsafe { SymbolCast { pointer: symbol }.symbol })
}

/// Return one bounded Apple semaphore retry sleep duration.
#[cfg(target_vendor = "apple")]
fn apple_semaphore_wait_sleep(deadline: Instant) -> Duration {
    let now = Instant::now();
    if now >= deadline {
        return Duration::ZERO;
    }

    let remaining = deadline.saturating_duration_since(now);

    remaining.min(APPLE_SEMAPHORE_WAIT_SLICE)
}

/// Wait for one semaphore permit with one bounded timeout.
pub(crate) fn unix_semaphore_wait_timed(
    semaphore: *mut libc::sem_t,
    timeout: Duration,
) -> Result<UnixSemaphoreWaitStatus, i32> {
    // prefer one monotonic timed wait on Linux when libc exports it
    #[cfg(target_os = "linux")]
    if let Some(sem_clockwait) = linux_sem_clockwait() {
        let deadline = absolute_monotonic_timespec(timeout)?;

        loop {
            let rc = unsafe {
                sem_clockwait(
                    semaphore,
                    libc::CLOCK_MONOTONIC,
                    &deadline as *const libc::timespec,
                )
            };
            if rc == 0 {
                return Ok(UnixSemaphoreWaitStatus::Acquired);
            }

            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }
            if errno == libc::ETIMEDOUT {
                return Ok(UnixSemaphoreWaitStatus::TimedOut);
            }

            return Err(errno);
        }
    }

    // otherwise use one preserved realtime deadline on hosts without monotonic semaphore waits
    #[cfg(not(target_vendor = "apple"))]
    {
        let deadline = SystemTime::now().checked_add(timeout).ok_or(libc::EINVAL)?;
        let deadline = absolute_realtime_timespec(deadline)?;

        loop {
            let rc = unsafe { libc::sem_timedwait(semaphore, &deadline as *const libc::timespec) };
            if rc == 0 {
                return Ok(UnixSemaphoreWaitStatus::Acquired);
            }

            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }
            if errno == libc::ETIMEDOUT {
                return Ok(UnixSemaphoreWaitStatus::TimedOut);
            }

            return Err(errno);
        }
    }

    // apple fallback
    #[cfg(target_vendor = "apple")]
    {
        let deadline = Instant::now().checked_add(timeout).ok_or(libc::EINVAL)?;

        loop {
            let rc = unsafe { libc::sem_trywait(semaphore) };
            if rc == 0 {
                return Ok(UnixSemaphoreWaitStatus::Acquired);
            }

            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }
            if errno == libc::EAGAIN {
                let sleep = apple_semaphore_wait_sleep(deadline);
                if sleep.is_zero() {
                    return Ok(UnixSemaphoreWaitStatus::TimedOut);
                }

                std::thread::sleep(sleep);
                continue;
            }

            return Err(errno);
        }
    }
}
