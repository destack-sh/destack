use std::time::Duration;
#[cfg(target_vendor = "apple")]
use std::time::Instant;
#[cfg(not(target_vendor = "apple"))]
use std::time::{SystemTime, UNIX_EPOCH};

use crate::platform::core as core_platform;

/// Poll interval for one Apple semaphore timed-wait fallback slice.
#[cfg(target_vendor = "apple")]
const APPLE_SEMAPHORE_WAIT_SLICE: Duration = Duration::from_millis(1);

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
fn absolute_realtime_timespec(timeout: Duration) -> Result<libc::timespec, i32> {
    let deadline = SystemTime::now().checked_add(timeout).ok_or(libc::EINVAL)?;
    let duration = deadline
        .duration_since(UNIX_EPOCH)
        .map_err(|_| libc::EINVAL)?;
    let seconds = i64::try_from(duration.as_secs()).map_err(|_| libc::EINVAL)?;

    Ok(libc::timespec {
        tv_sec: seconds as libc::time_t,
        tv_nsec: duration.subsec_nanos() as libc::c_long,
    })
}

/// Wait for one semaphore permit with one bounded timeout.
pub(crate) fn unix_semaphore_wait_timed(
    semaphore: *mut libc::sem_t,
    timeout: Duration,
) -> Result<UnixSemaphoreWaitStatus, i32> {
    #[cfg(not(target_vendor = "apple"))]
    loop {
        let deadline = absolute_realtime_timespec(timeout)?;
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

    #[cfg(target_vendor = "apple")]
    {
        let started_at = Instant::now();

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
                if started_at.elapsed() >= timeout {
                    return Ok(UnixSemaphoreWaitStatus::TimedOut);
                }

                std::thread::sleep(APPLE_SEMAPHORE_WAIT_SLICE);
                continue;
            }

            return Err(errno);
        }
    }
}
