use destack_workspace::{ExecutionMode, RandomMode, RuntimeOptions};

use crate::diagnostic::RuntimeStatus;
use crate::platform::PlatformContext;
use crate::platform::random::{
    RandomStream, destack_random_next_u64, destack_random_next_u64_from,
};
use crate::platform::resource::{ListenerHandle, ResourceKind};
use crate::runtime::{Runtime, RuntimeCallContext, RuntimeContext, enter_runtime_call_context};

/// Runtime harness for runtime tests.
pub(crate) struct TestRuntime {
    /// Runtime under test.
    pub runtime: Runtime,
}

impl TestRuntime {
    /// Build a runtime with deterministic random settings.
    pub(crate) fn deterministic_random() -> Self {
        // deterministic random options
        let mut options = RuntimeOptions::default();
        options.execution_mode = ExecutionMode::Fast;
        options.random.mode = RandomMode::Deterministic;
        options.random.seed = Some(0);

        // runtime with deterministic random state
        let context =
            RuntimeContext::from_runtime_options(PlatformContext::new(Vec::new()), &options);
        let runtime = Runtime::new(context);

        Self { runtime }
    }

    /// Execute a native binding within a runtime call context.
    pub(crate) fn with_native_call_context<T>(
        &self,
        run: impl FnOnce(&RuntimeCallContext) -> T,
    ) -> T {
        // enter a native call context for the binding
        let call_context = RuntimeCallContext::new(
            &self.runtime.context,
            self.runtime.scheduler.as_ref(),
            self.runtime.bindings.policy(),
        );
        let _guard = enter_runtime_call_context(&call_context);

        // run the native call
        run(&call_context)
    }

    /// Execute the native random binding with deterministic runtime state.
    pub(crate) fn call_native_next_u64(&self) -> u64 {
        self.with_native_call_context(|_| {
            let mut out = 0u64;
            let status = unsafe { destack_random_next_u64(&mut out) };
            assert_eq!(status, RuntimeStatus::OK);
            out
        })
    }

    /// Execute the native stream binding with deterministic runtime state.
    pub(crate) fn call_native_next_u64_from(&self, stream: u64) -> u64 {
        self.with_native_call_context(|_| {
            let mut out = 0u64;
            let status = unsafe { destack_random_next_u64_from(&mut out, RandomStream(stream)) };
            assert_eq!(status, RuntimeStatus::OK);
            out
        })
    }

    /// Assert a RuntimeStatus is OK and include the runtime error message if not.
    pub(crate) fn assert_status_ok(&self, status: RuntimeStatus, label: &str) {
        if status == RuntimeStatus::OK {
            return;
        }

        let error = self
            .runtime
            .context
            .errors()
            .take(crate::diagnostic::RuntimeErrorId::from_raw(status.error_id))
            .map(|error| error.message())
            .unwrap_or_else(|| "missing runtime error".to_string());

        panic!("{label} failed: {error}");
    }

    /// Assert a RuntimeStatus is not OK.
    pub(crate) fn assert_status_err(&self, status: RuntimeStatus, label: &str) {
        if status != RuntimeStatus::OK {
            return;
        }

        let error = self
            .runtime
            .context
            .errors()
            .take(crate::diagnostic::RuntimeErrorId::from_raw(status.error_id))
            .map(|error| error.message())
            .unwrap_or_else(|| "missing runtime error".to_string());

        panic!("{label} unexpectedly succeeded: {error}");
    }

    /// Read the port assigned to a listener handle.
    #[cfg(unix)]
    pub(crate) fn listener_port(&self, handle: ListenerHandle) -> u16 {
        let fd = self
            .runtime
            .context
            .resources()
            .with_entry(handle.0, |entry| {
                if entry.kind != ResourceKind::Listener {
                    return None;
                }
                entry.fd()
            })
            .flatten()
            .expect("listener handle must be valid");

        let mut storage = std::mem::MaybeUninit::<libc::sockaddr_storage>::uninit();
        let mut length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        let rc = unsafe { libc::getsockname(fd, storage.as_mut_ptr() as *mut _, &mut length) };
        assert_eq!(rc, 0, "getsockname failed");
        let storage = unsafe { storage.assume_init() };
        match storage.ss_family as libc::c_int {
            libc::AF_INET => {
                let addr = unsafe { &*(std::ptr::addr_of!(storage) as *const libc::sockaddr_in) };
                u16::from_be(addr.sin_port)
            }
            libc::AF_INET6 => {
                let addr = unsafe { &*(std::ptr::addr_of!(storage) as *const libc::sockaddr_in6) };
                u16::from_be(addr.sin6_port)
            }
            _ => panic!("unsupported listener address family"),
        }
    }

    /// Read the port assigned to a listener handle.
    #[cfg(windows)]
    pub(crate) fn listener_port(&self, handle: ListenerHandle) -> u16 {
        use windows_sys::Win32::Networking::WinSock::{
            AF_INET, AF_INET6, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_STORAGE, SOCKET,
            getsockname,
        };

        let socket = self
            .runtime
            .context
            .resources()
            .with_entry(handle.0, |entry| {
                if entry.kind != ResourceKind::Listener {
                    return None;
                }
                entry.socket()
            })
            .flatten()
            .expect("listener handle must be valid") as SOCKET;

        let mut storage = std::mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
        let mut length = std::mem::size_of::<SOCKADDR_STORAGE>() as i32;
        let rc = unsafe { getsockname(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
        assert_eq!(rc, 0, "getsockname failed");
        let storage = unsafe { storage.assume_init() };
        match storage.ss_family as i32 {
            AF_INET => {
                let addr = unsafe { &*(std::ptr::addr_of!(storage) as *const SOCKADDR_IN) };
                u16::from_be(addr.sin_port)
            }
            AF_INET6 => {
                let addr = unsafe { &*(std::ptr::addr_of!(storage) as *const SOCKADDR_IN6) };
                u16::from_be(addr.sin6_port)
            }
            _ => panic!("unsupported listener address family"),
        }
    }
}
