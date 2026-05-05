#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;

#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::sync::atomic::Ordering;
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::sync::mpsc;
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::thread;
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use std::time::Duration;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
pub(crate) use crate::tests::platform::assert_platform_error_code;
use crate::tests::runtime::TestRuntime;

/// Return the raw address for one atomic 32 bit word.
pub(crate) fn atomic_word_address(word: &AtomicU32) -> u64 {
    word.as_ptr() as u64
}

/// Wake one or all waiters for one test address.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn wake_test_address(address: u64, wake_all: bool) {
    const FUTEX_WAKE_PRIVATE_OPERATION: libc::c_int = 129;

    let wake_count = if wake_all { i32::MAX } else { 1_i32 };
    let address = address as usize as *const u32;

    let rc = unsafe {
        libc::syscall(
            libc::SYS_futex,
            address,
            FUTEX_WAKE_PRIVATE_OPERATION,
            wake_count,
            std::ptr::null::<libc::timespec>(),
            std::ptr::null::<libc::c_void>(),
            0_usize,
        )
    };
    assert!(rc >= 0, "futex wake helper failed");
}

/// Wake one or all waiters for one test address.
#[cfg(windows)]
pub(crate) fn wake_test_address(address: u64, wake_all: bool) {
    let address = address as usize as *const u32;

    unsafe {
        if wake_all {
            windows_sys::Win32::System::Threading::WakeByAddressAll(
                address as *const std::ffi::c_void,
            );
        } else {
            windows_sys::Win32::System::Threading::WakeByAddressSingle(
                address as *const std::ffi::c_void,
            );
        }
    }
}

/// Spawn one helper thread that mutates and wakes one atomic word.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
pub(crate) fn spawn_wake_thread(
    word: Arc<AtomicU32>,
    next_value: u32,
    delay: Duration,
    wake_all: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        thread::sleep(delay);
        word.store(next_value, Ordering::Release);
        wake_test_address(atomic_word_address(&word), wake_all);
    })
}

/// Test harness context used by tests.
pub(crate) struct ThreadHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(super) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(super) vm_context: Option<*mut ()>,
}

/// Native thread harness.
pub(crate) struct NativeThreadHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeThreadHarness {
    /// Create a new native thread harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM thread harness.
pub(crate) struct VmThreadHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmThreadHarness {
    /// Create a new VM thread harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness kind used for helper threads.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
#[derive(Clone, Copy)]
pub(crate) enum ThreadHarnessKind {
    /// Native harness kind.
    Native,
    /// VM harness kind.
    Vm,
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum ThreadHarnessHandle {
    /// Native thread harness.
    Native(NativeThreadHarness),
    /// VM thread harness.
    Vm(VmThreadHarness),
}

impl ThreadHarnessHandle {
    /// Return the concrete harness kind.
    #[cfg(any(target_os = "linux", target_os = "android", windows))]
    pub(crate) fn kind(&self) -> ThreadHarnessKind {
        match self {
            ThreadHarnessHandle::Native(_) => ThreadHarnessKind::Native,
            ThreadHarnessHandle::Vm(_) => ThreadHarnessKind::Vm,
        }
    }

    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(ThreadHarnessContext<'call>) -> R,
    {
        match self {
            ThreadHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(ThreadHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            ThreadHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(ThreadHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&mut self, callback: F)
    where
        F: for<'call> FnOnce(ThreadHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("thread harness call should succeed");
    }
}

/// Run one callback against one selected harness kind.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
pub(crate) fn with_harness_kind_context<F>(
    kind: ThreadHarnessKind,
    callback: F,
) -> RuntimeResult<()>
where
    F: for<'call> FnOnce(ThreadHarnessContext<'call>) -> RuntimeResult<()>,
{
    match kind {
        ThreadHarnessKind::Native => {
            let harness = ThreadHarnessHandle::Native(NativeThreadHarness::new());
            harness.with_context(callback)
        }
        ThreadHarnessKind::Vm => {
            let harness = ThreadHarnessHandle::Vm(VmThreadHarness::new());
            harness.with_context(callback)
        }
    }
}

/// Spawn one helper waiter that blocks on the thread wait binding.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
pub(crate) fn spawn_wait_thread(
    kind: ThreadHarnessKind,
    ready_count: Arc<AtomicU32>,
    address: u64,
    expected: u32,
    timeout: Duration,
    sender: mpsc::Sender<RuntimeResult<()>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        ready_count.fetch_add(1, Ordering::AcqRel);

        let result = with_harness_kind_context(kind, |mut context| {
            context.destack_thread_address_wait(address, expected, timeout.as_nanos() as u64)
        });

        sender
            .send(result)
            .expect("thread wait helper receiver should stay alive");
    })
}

/// Receive one thread wait result within one bounded time window.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
pub(crate) fn recv_wait_result(
    receiver: &mpsc::Receiver<RuntimeResult<()>>,
    timeout: Duration,
) -> Option<RuntimeResult<()>> {
    receiver.recv_timeout(timeout).ok()
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut ThreadHarnessHandle),
{
    let mut native = ThreadHarnessHandle::Native(NativeThreadHarness::new());
    callback(&mut native);
    let mut vm = ThreadHarnessHandle::Vm(VmThreadHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(ThreadHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
