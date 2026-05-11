use std::time::Duration;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::time::Instant;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{io_error, posix_name, register_semaphore, semaphore_pointer, timed_out};
#[cfg(any(target_os = "linux", target_os = "android"))]
use super::core::{io_error_with_errno, shared_memory_descriptor, would_block};

/// Create one named semaphore.
const SEMAPHORE_CREATE_OPERATION: &str = "destack.ipc.sync.semaphoreCreate";
/// Increment one semaphore count.
const SEMAPHORE_POST_OPERATION: &str = "destack.ipc.sync.semaphorePost";
/// Wait one semaphore count.
const SEMAPHORE_WAIT_OPERATION: &str = "destack.ipc.sync.semaphoreWait";
/// Wait one futex word.
const FUTEX_WAIT_OPERATION: &str = "destack.ipc.sync.futexWait";
/// Wake futex waiters.
const FUTEX_WAKE_OPERATION: &str = "destack.ipc.sync.futexWake";
/// Futex wait operation code for shared mappings.
#[cfg(any(target_os = "linux", target_os = "android"))]
const FUTEX_WAIT_OPERATION_CODE: libc::c_int = 0;
/// Futex wake operation code for shared mappings.
#[cfg(any(target_os = "linux", target_os = "android"))]
const FUTEX_WAKE_OPERATION_CODE: libc::c_int = 1;

/// Mapping descriptor for one futex word inside shared memory.
#[cfg(any(target_os = "linux", target_os = "android"))]
struct FutexWordMapping {
    /// Base mapping address returned by mmap.
    base: *mut libc::c_void,
    /// Mapping byte length used for munmap.
    length: usize,
    /// Futex word address within the mapping.
    word: *mut u32,
}

/// Convert one duration into one relative host timespec.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn relative_timespec(timeout: Duration) -> RuntimeResult<libc::timespec> {
    let seconds = i64::try_from(timeout.as_secs()).map_err(|_| {
        core_platform::invalid_argument("timeoutNs", "timeout is too large for host timespec range")
    })?;

    Ok(libc::timespec {
        tv_sec: seconds as libc::time_t,
        tv_nsec: timeout.subsec_nanos() as libc::c_long,
    })
}

/// Map one futex word from one shared-memory object and offset.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn map_futex_word(
    binding: &BindingCallContext,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    operation: &'static str,
) -> RuntimeResult<FutexWordMapping> {
    // validate futex alignment requirements
    if !offset.is_multiple_of(std::mem::size_of::<u32>() as u64) {
        return Err(core_platform::invalid_argument(
            "offset",
            "offset must be aligned to 4 bytes",
        ));
    }

    // resolve one shared-memory descriptor
    let descriptor = shared_memory_descriptor(binding, sharedmemory, operation)?;

    // validate shared-memory size against the requested offset
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    let status = unsafe { libc::fstat(descriptor, metadata.as_mut_ptr()) };
    if status != 0 {
        return Err(io_error(
            operation,
            "fstat",
            "failed to inspect shared-memory size",
        ));
    }

    let metadata = unsafe { metadata.assume_init() };
    if metadata.st_size < 0 {
        return Err(core_platform::invalid_argument(
            "sharedMemory",
            "shared-memory size is negative on host metadata",
        ));
    }
    let metadata_size = u64::try_from(metadata.st_size).map_err(|_| {
        core_platform::invalid_argument("sharedMemory", "shared-memory size exceeds u64 range")
    })?;

    let required = offset
        .checked_add(std::mem::size_of::<u32>() as u64)
        .ok_or_else(|| {
            core_platform::invalid_argument("offset", "offset overflowed futex word bounds")
        })?;
    if required > metadata_size {
        return Err(core_platform::invalid_argument(
            "offset",
            "offset falls outside the shared-memory object",
        ));
    }

    // compute one page-aligned mapping window for the futex word
    let page_size_raw = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size_raw <= 0 {
        return Err(io_error(
            operation,
            "sysconf",
            "failed to resolve host page size",
        ));
    }

    let page_size = page_size_raw as u64;
    let page_mask = !(page_size - 1);
    let map_offset = offset & page_mask;
    let offset_delta = usize::try_from(offset - map_offset).map_err(|_| {
        core_platform::invalid_argument("offset", "offset exceeds host usize range")
    })?;
    let map_length = offset_delta
        .checked_add(std::mem::size_of::<u32>())
        .ok_or_else(|| {
            core_platform::invalid_argument("offset", "offset mapping length overflowed")
        })?;
    let map_offset_host = i64::try_from(map_offset).map_err(|_| {
        core_platform::invalid_argument("offset", "offset exceeds host off_t range")
    })?;

    // map one writable view for futex wait or wake operations
    let base = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            map_length,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            descriptor,
            map_offset_host as libc::off_t,
        )
    };
    if base == libc::MAP_FAILED {
        return Err(io_error(
            operation,
            "mmap",
            "failed to map shared-memory futex word",
        ));
    }

    let word = unsafe { (base as *mut u8).add(offset_delta) as *mut u32 };
    Ok(FutexWordMapping {
        base,
        length: map_length,
        word,
    })
}

/// Unmap one mapped futex word view.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn unmap_futex_word(mapping: &FutexWordMapping) {
    unsafe {
        libc::munmap(mapping.base, mapping.length);
    }
}

/// Wait on one shared-memory futex word.
pub(crate) unsafe fn destack_ipc_futex_wait(
    binding: &BindingCallContext,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // map one futex word view from shared-memory state
        let mapping = map_futex_word(binding, sharedmemory, offset, FUTEX_WAIT_OPERATION)?;

        // execute one futex wait loop with timeout and signal handling
        let result = if timeoutns == u64::MAX {
            loop {
                let rc = unsafe {
                    libc::syscall(
                        libc::SYS_futex,
                        mapping.word,
                        FUTEX_WAIT_OPERATION_CODE,
                        expected as libc::c_int,
                        std::ptr::null::<libc::timespec>(),
                        std::ptr::null::<libc::c_void>(),
                        0_usize,
                    )
                };
                if rc == 0 {
                    break Ok(());
                }

                let errno = core_platform::get_errno();
                if errno == libc::EINTR {
                    continue;
                }
                if errno == libc::EAGAIN {
                    break Err(would_block(
                        FUTEX_WAIT_OPERATION,
                        "failed to wait futex: value no longer matched expected",
                    ));
                }

                break Err(io_error_with_errno(
                    FUTEX_WAIT_OPERATION,
                    "futex",
                    errno,
                    "failed to wait futex",
                ));
            }
        } else {
            let timeout = Duration::from_nanos(timeoutns);
            let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
                core_platform::invalid_argument("timeoutNs", "timeout overflowed host deadline")
            })?;

            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                let timeout = relative_timespec(remaining)?;
                let rc = unsafe {
                    libc::syscall(
                        libc::SYS_futex,
                        mapping.word,
                        FUTEX_WAIT_OPERATION_CODE,
                        expected as libc::c_int,
                        &timeout as *const libc::timespec,
                        std::ptr::null::<libc::c_void>(),
                        0_usize,
                    )
                };
                if rc == 0 {
                    break Ok(());
                }

                let errno = core_platform::get_errno();
                if errno == libc::EINTR {
                    if Instant::now() >= deadline {
                        if timeoutns == 0 {
                            break Err(would_block(
                                FUTEX_WAIT_OPERATION,
                                "failed to wait futex: no wake observed",
                            ));
                        }

                        break Err(timed_out(
                            FUTEX_WAIT_OPERATION,
                            "failed to wait futex: timed out waiting for wake",
                        ));
                    }

                    continue;
                }
                if errno == libc::EAGAIN {
                    break Err(would_block(
                        FUTEX_WAIT_OPERATION,
                        "failed to wait futex: value no longer matched expected",
                    ));
                }
                if errno == libc::ETIMEDOUT {
                    if timeoutns == 0 {
                        break Err(would_block(
                            FUTEX_WAIT_OPERATION,
                            "failed to wait futex: no wake observed",
                        ));
                    }

                    break Err(timed_out(
                        FUTEX_WAIT_OPERATION,
                        "failed to wait futex: timed out waiting for wake",
                    ));
                }

                break Err(io_error_with_errno(
                    FUTEX_WAIT_OPERATION,
                    "futex",
                    errno,
                    "failed to wait futex",
                ));
            }
        };

        // always unmap one futex word view
        unmap_futex_word(&mapping);
        result
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let _ = (binding, sharedmemory, offset, expected, timeoutns);

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    Err(core_platform::not_supported(FUTEX_WAIT_OPERATION))
}

/// Wake futex waiters for one shared-memory word.
pub(crate) unsafe fn destack_ipc_futex_wake(
    binding: &BindingCallContext,
    out: *mut u32,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // reject impossible futex wake-count values for host c_int ranges
        let count = i32::try_from(count).map_err(|_| {
            core_platform::invalid_argument("count", "count exceeds host futex range")
        })?;

        // map one futex word view from shared-memory state
        let mapping = map_futex_word(binding, sharedmemory, offset, FUTEX_WAKE_OPERATION)?;

        // wake one or more waiters blocked on the futex word
        let woken = unsafe {
            libc::syscall(
                libc::SYS_futex,
                mapping.word,
                FUTEX_WAKE_OPERATION_CODE,
                count as libc::c_int,
                std::ptr::null::<libc::timespec>(),
                std::ptr::null::<libc::c_void>(),
                0_usize,
            )
        };
        if woken < 0 {
            let errno = core_platform::get_errno();
            unmap_futex_word(&mapping);
            return Err(io_error_with_errno(
                FUTEX_WAKE_OPERATION,
                "futex",
                errno,
                "failed to wake futex waiters",
            ));
        }

        // always unmap one futex word view
        unmap_futex_word(&mapping);

        // write the host-reported wake count
        unsafe {
            out.write(woken as u32);
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let _ = (binding, sharedmemory, offset, count);

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    Err(core_platform::not_supported(FUTEX_WAKE_OPERATION))
}

/// Create one named semaphore.
pub(crate) unsafe fn destack_ipc_semaphore_create(
    binding: &BindingCallContext,
    out: *mut resource::SemaphoreHandle,
    name: NativeStringRef,
    initial: u32,
    flags: u32,
) -> RuntimeResult<()> {
    // validate output and input flags
    core_platform::ensure_out(out, "out")?;
    core_platform::ensure_zero_flags(flags, "flags")?;

    // decode and normalize the semaphore name
    let name = posix_name(name, "name")?;

    // create one named semaphore object
    let semaphore =
        unsafe { libc::sem_open(name.as_ptr(), libc::O_CREAT | libc::O_EXCL, 0o600, initial) };
    if semaphore == libc::SEM_FAILED {
        return Err(io_error(
            SEMAPHORE_CREATE_OPERATION,
            "sem_open",
            "failed to create semaphore",
        ));
    }

    // register semaphore state and write handle output
    let handle = register_semaphore(binding, semaphore);
    unsafe {
        out.write(handle);
    }

    Ok(())
}

/// Increment one semaphore count.
pub(crate) unsafe fn destack_ipc_semaphore_post(
    binding: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    // resolve one semaphore pointer
    let semaphore = semaphore_pointer(binding, handle, SEMAPHORE_POST_OPERATION)?;

    // post one permit for each requested count
    for _ in 0..count {
        let rc = unsafe { libc::sem_post(semaphore) };
        if rc == 0 {
            continue;
        }

        return Err(io_error(
            SEMAPHORE_POST_OPERATION,
            "sem_post",
            "failed to post semaphore",
        ));
    }

    Ok(())
}

/// Wait one semaphore count.
pub(crate) unsafe fn destack_ipc_semaphore_wait(
    binding: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    // resolve one semaphore pointer
    let semaphore = semaphore_pointer(binding, handle, SEMAPHORE_WAIT_OPERATION)?;

    // handle immediate try-wait mode
    if timeoutns == 0 {
        let rc = unsafe { libc::sem_trywait(semaphore) };
        if rc == 0 {
            return Ok(());
        }

        let errno = core_platform::get_errno();
        if errno == libc::EAGAIN {
            return Err(timed_out(
                SEMAPHORE_WAIT_OPERATION,
                "failed to wait semaphore: no permit was available",
            ));
        }

        return Err(io_error(
            SEMAPHORE_WAIT_OPERATION,
            "sem_trywait",
            "failed to try-wait semaphore",
        ));
    }

    // handle infinite wait mode
    if timeoutns == u64::MAX {
        loop {
            let rc = unsafe { libc::sem_wait(semaphore) };
            if rc == 0 {
                return Ok(());
            }

            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }

            return Err(io_error(
                SEMAPHORE_WAIT_OPERATION,
                "sem_wait",
                "failed to wait semaphore",
            ));
        }
    }

    let timeout = Duration::from_nanos(timeoutns);
    match core_platform::unix_semaphore_wait_timed(semaphore, timeout) {
        Ok(core_platform::UnixSemaphoreWaitStatus::Acquired) => Ok(()),
        Ok(core_platform::UnixSemaphoreWaitStatus::TimedOut) => Err(timed_out(
            SEMAPHORE_WAIT_OPERATION,
            "failed to wait semaphore: timed out",
        )),
        Err(_) => Err(io_error(
            SEMAPHORE_WAIT_OPERATION,
            "sem_wait",
            "failed to timed-wait semaphore",
        )),
    }
}
