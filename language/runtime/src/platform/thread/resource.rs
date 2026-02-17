#[cfg(any(unix, windows))]
use std::cell::UnsafeCell;
#[cfg(windows)]
use std::collections::HashMap;

#[cfg(windows)]
use parking_lot::Mutex;

/// Marker payload for one thread-local key resource.
#[derive(Debug)]
pub(crate) struct ThreadLocalResource {
    /// Native host TLS key.
    #[cfg(unix)]
    pub(crate) key: libc::pthread_key_t,
    /// Native host TLS key.
    #[cfg(windows)]
    pub(crate) key: u32,
    /// Native host TLS key.
    #[cfg(not(any(unix, windows)))]
    pub(crate) key: u64,
}

/// Shared payload for one spawned thread handle.
pub(crate) struct ThreadResource {
    /// Native host thread handle used for join and detach operations.
    #[cfg(unix)]
    pub(crate) native_handle: libc::pthread_t,
    /// Native host thread handle used for join and detach operations.
    #[cfg(windows)]
    pub(crate) native_handle: windows_sys::Win32::Foundation::HANDLE,
}

/// Native mutex payload.
#[cfg(unix)]
pub(crate) struct MutexResource {
    /// Native pthread mutex object.
    pub(crate) mutex: UnsafeCell<libc::pthread_mutex_t>,
}

/// Native mutex payload.
#[cfg(windows)]
pub(crate) struct MutexResource {
    /// Native critical section object.
    pub(crate) critical_section:
        UnsafeCell<windows_sys::Win32::System::Threading::CRITICAL_SECTION>,
}

#[cfg(windows)]
unsafe impl Send for MutexResource {}
#[cfg(windows)]
unsafe impl Sync for MutexResource {}
#[cfg(unix)]
unsafe impl Send for MutexResource {}
#[cfg(unix)]
unsafe impl Sync for MutexResource {}

/// Native read-write lock payload.
#[cfg(unix)]
pub(crate) struct RwLockResource {
    /// Native pthread rwlock object.
    pub(crate) rwlock: UnsafeCell<libc::pthread_rwlock_t>,
}

/// Native read-write lock payload.
#[cfg(windows)]
pub(crate) struct RwLockResource {
    /// Native SRW lock object.
    pub(crate) rwlock: UnsafeCell<windows_sys::Win32::System::Threading::SRWLOCK>,
    /// Ownership bookkeeping required for unlock dispatch.
    pub(crate) ownership: Mutex<WindowsRwLockOwnership>,
}

#[cfg(windows)]
unsafe impl Send for RwLockResource {}
#[cfg(windows)]
unsafe impl Sync for RwLockResource {}
#[cfg(unix)]
unsafe impl Send for RwLockResource {}
#[cfg(unix)]
unsafe impl Sync for RwLockResource {}

/// Thread ownership metadata for SRW release mode dispatch.
#[cfg(windows)]
#[derive(Debug, Default)]
pub(crate) struct WindowsRwLockOwnership {
    /// Current writer owner, if any.
    pub(crate) writer: Option<u64>,
    /// Current reader owner counts.
    pub(crate) readers: HashMap<u64, u32>,
}

/// Native condition variable payload.
#[cfg(unix)]
pub(crate) struct CondVarResource {
    /// Native pthread condition variable object.
    pub(crate) condvar: UnsafeCell<libc::pthread_cond_t>,
}

/// Native condition variable payload.
#[cfg(windows)]
pub(crate) struct CondVarResource {
    /// Native condition variable object.
    pub(crate) condvar: UnsafeCell<windows_sys::Win32::System::Threading::CONDITION_VARIABLE>,
}

#[cfg(windows)]
unsafe impl Send for CondVarResource {}
#[cfg(windows)]
unsafe impl Sync for CondVarResource {}
#[cfg(unix)]
unsafe impl Send for CondVarResource {}
#[cfg(unix)]
unsafe impl Sync for CondVarResource {}

/// Native semaphore payload.
#[cfg(unix)]
pub(crate) struct SemaphoreResource {
    /// Native semaphore object.
    pub(crate) semaphore: UnsafeCell<libc::sem_t>,
    /// Maximum permit count.
    pub(crate) maximum: u32,
}

/// Native semaphore payload.
#[cfg(windows)]
pub(crate) struct SemaphoreResource {
    /// Native semaphore handle.
    pub(crate) semaphore: windows_sys::Win32::Foundation::HANDLE,
}

/// Native barrier payload.
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
pub(crate) struct BarrierResource {
    /// Native pthread barrier object.
    pub(crate) barrier: UnsafeCell<libc::pthread_barrier_t>,
    /// Participant count for fast non-blocking validation.
    pub(crate) participants: u32,
}

/// Native barrier payload.
#[cfg(windows)]
pub(crate) struct BarrierResource {
    /// Native synchronization barrier object.
    pub(crate) barrier: UnsafeCell<windows_sys::Win32::System::Threading::SYNCHRONIZATION_BARRIER>,
    /// Participant count for fast non-blocking validation.
    pub(crate) participants: u32,
}

#[cfg(windows)]
unsafe impl Send for BarrierResource {}
#[cfg(windows)]
unsafe impl Sync for BarrierResource {}
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
unsafe impl Send for BarrierResource {}
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
unsafe impl Sync for BarrierResource {}
#[cfg(unix)]
unsafe impl Send for SemaphoreResource {}
#[cfg(unix)]
unsafe impl Sync for SemaphoreResource {}

#[cfg(unix)]
impl Drop for MutexResource {
    fn drop(&mut self) {
        unsafe {
            libc::pthread_mutex_destroy(self.mutex.get());
        }
    }
}

#[cfg(windows)]
impl Drop for MutexResource {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Threading::DeleteCriticalSection(
                self.critical_section.get(),
            );
        }
    }
}

#[cfg(unix)]
impl Drop for RwLockResource {
    fn drop(&mut self) {
        unsafe {
            libc::pthread_rwlock_destroy(self.rwlock.get());
        }
    }
}

#[cfg(unix)]
impl Drop for CondVarResource {
    fn drop(&mut self) {
        unsafe {
            libc::pthread_cond_destroy(self.condvar.get());
        }
    }
}

#[cfg(unix)]
impl Drop for SemaphoreResource {
    fn drop(&mut self) {
        #[allow(deprecated)]
        unsafe {
            libc::sem_destroy(self.semaphore.get());
        }
    }
}

#[cfg(windows)]
impl Drop for SemaphoreResource {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.semaphore);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
impl Drop for BarrierResource {
    fn drop(&mut self) {
        unsafe {
            libc::pthread_barrier_destroy(self.barrier.get());
        }
    }
}

#[cfg(windows)]
impl Drop for BarrierResource {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Threading::DeleteSynchronizationBarrier(self.barrier.get());
        }
    }
}
