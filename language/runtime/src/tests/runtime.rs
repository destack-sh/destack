use destack_base::LocalStringPool;
use destack_mir::NodeTree;
use destack_vm as vm;
use destack_workspace::{ExecutionMode, RandomMode, RandomOptions, RuntimeOptions};

use crate::diagnostic::{RuntimeError, RuntimeErrorId, RuntimeResult, RuntimeStatus};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::random::{
    RandomStream, destack_random_stream_next_u64, destack_random_stream_next_u64_from,
};
use crate::platform::resource::{ListenerHandle, ResourceKind};
use crate::platform::{PlatformContext, PlatformError};
use crate::runtime::{Runtime, RuntimeCallContext, RuntimeContext, enter_runtime_call_context};

/// Runtime harness for runtime tests.
#[cfg_attr(windows, allow(dead_code))]
pub(crate) struct TestRuntime {
    /// Runtime under test.
    pub runtime: Runtime,
    /// VM isolate backing VM bindings in tests.
    vm_isolate: std::cell::RefCell<vm::Isolate>,
}

#[cfg_attr(windows, allow(dead_code))]
impl TestRuntime {
    /// Build a runtime with deterministic random settings.
    pub(crate) fn deterministic_random() -> Self {
        // deterministic random options
        let options = RuntimeOptions {
            execution_mode: ExecutionMode::Fast,
            random: RandomOptions {
                mode: RandomMode::Deterministic,
                seed: Some(0),
                ..RandomOptions::default()
            },
            ..RuntimeOptions::default()
        };

        // runtime with deterministic random state
        let context =
            RuntimeContext::from_runtime_options(PlatformContext::new(Vec::new()), &options);
        let runtime = Runtime::new(context);

        let tree = NodeTree::new();
        let strings = LocalStringPool::new().into_immutable();
        let vm_isolate = vm::Isolate::new(tree, strings).expect("test vm isolate should build");

        Self {
            runtime,
            vm_isolate: std::cell::RefCell::new(vm_isolate),
        }
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

    /// Execute a VM binding within a runtime call context.
    pub(crate) fn with_vm_call_context<T>(
        &self,
        run: impl for<'ctx> FnOnce(&RuntimeCallContext, &mut vm::RuntimeContext<'ctx>) -> T,
    ) -> T {
        // run the VM call with a fresh runtime call context
        let mut isolate = self.vm_isolate.borrow_mut();
        isolate.with_runtime_context(|context| {
            let call_context = RuntimeCallContext::new(
                &self.runtime.context,
                self.runtime.scheduler.as_ref(),
                self.runtime.bindings.policy(),
            );
            let _guard = enter_runtime_call_context(&call_context);
            run(&call_context, context)
        })
    }

    /// Execute the native random binding with deterministic runtime state.
    pub(crate) fn call_native_next_u64(&self) -> RuntimeResult<u64> {
        self.with_native_call_context(|_| {
            let mut out = 0u64;
            let status = unsafe { destack_random_stream_next_u64(&mut out) };
            self.status_value(status, "random stream nextU64", out)
        })
    }

    /// Execute the native stream binding with deterministic runtime state.
    pub(crate) fn call_native_next_u64_from(&self, stream: u64) -> RuntimeResult<u64> {
        self.with_native_call_context(|_| {
            let mut out = 0u64;
            let status =
                unsafe { destack_random_stream_next_u64_from(&mut out, RandomStream(stream)) };
            self.status_value(status, "random stream nextU64From", out)
        })
    }

    /// Convert a native status and value into a runtime result.
    fn status_value<T>(&self, status: RuntimeStatus, label: &str, value: T) -> RuntimeResult<T> {
        // return the produced value on success
        if status == RuntimeStatus::OK {
            return Ok(value);
        }

        // synthesize a diagnostic when the runtime status lacks an error id
        if status.error_id == 0 {
            if status.code == PlatformErrorCode::NotSupported.number().saturating_add(1) {
                let error = RuntimeError::from(PlatformError::not_supported(label));
                return Err(error.boxed());
            }

            let error = RuntimeError::from(PlatformError::io(format!(
                "{label} failed without runtime error id",
            )));
            return Err(error.boxed());
        }

        // take the stored runtime error
        let error_id = RuntimeErrorId::from_raw(status.error_id);
        let error = self
            .runtime
            .context
            .errors()
            .take(error_id)
            .unwrap_or_else(|| {
                RuntimeError::from(PlatformError::io(format!(
                    "{label} failed with missing runtime error",
                )))
                .boxed()
            });

        Err(error)
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
            .take(RuntimeErrorId::from_raw(status.error_id))
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
            .take(RuntimeErrorId::from_raw(status.error_id))
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
            value if value == AF_INET as i32 => {
                let addr = unsafe { &*(std::ptr::addr_of!(storage) as *const SOCKADDR_IN) };
                u16::from_be(addr.sin_port)
            }
            value if value == AF_INET6 as i32 => {
                let addr = unsafe { &*(std::ptr::addr_of!(storage) as *const SOCKADDR_IN6) };
                u16::from_be(addr.sin6_port)
            }
            _ => panic!("unsupported listener address family"),
        }
    }
}
