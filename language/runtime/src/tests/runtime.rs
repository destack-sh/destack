use destack_core::LocalStringPool;
use destack_mir::NodeTree;
use destack_vm as vm;
use destack_workspace::{ExecutionMode, RandomMode, RandomOptions, RuntimeOptions};

#[cfg(test)]
use crate::diagnostic::{DiagnosticId, RuntimeError, RuntimeResult, RuntimeStatus};
use crate::host::HostSession;
#[cfg(test)]
use crate::platform::PlatformError;
#[cfg(test)]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(test)]
use crate::platform::random::{
    RandomStream, destack_random_stream_next_u64, destack_random_stream_next_u64_from,
};
#[cfg(test)]
use crate::platform::resource::SocketHandle;
#[cfg(test)]
use crate::platform::resource::{ListenerHandle, ResourceKind};
use crate::runtime::{
    Agent, BindingCallContext, World, enter_binding_call_context, enter_current_agent_context,
};

/// Runtime harness for runtime tests.
pub(crate) struct TestRuntime {
    /// Shared world that owns the agent lifetime.
    world: std::sync::Arc<World>,
    /// Agent under test.
    pub agent: Box<Agent>,
    /// Host under test.
    host: HostSession,
    /// VM isolate backing VM bindings in tests.
    vm_isolate: std::cell::RefCell<vm::Isolate>,
    /// Heap backing the VM isolate in tests.
    vm_heap: std::cell::RefCell<vm::Heap>,
    /// Shared memory backing the VM isolate in tests.
    vm_shared: std::cell::RefCell<vm::SharedSpace>,
}

impl TestRuntime {
    /// Build a runtime with deterministic random settings.
    pub(crate) fn deterministic_random() -> Self {
        Self::deterministic_random_with_options(|_| {})
    }

    /// Build a runtime with deterministic random settings and one options mutator.
    pub(crate) fn deterministic_random_with_options(
        configure: impl FnOnce(&mut RuntimeOptions),
    ) -> Self {
        // build baseline deterministic options
        let mut options = Self::deterministic_runtime_options();
        configure(&mut options);

        // runtime with deterministic random state
        Self::from_runtime_options(options)
    }

    /// Build the baseline deterministic runtime options.
    fn deterministic_runtime_options() -> RuntimeOptions {
        // route host-backed crypto snapshots to deterministic local files
        let process_id = std::process::id();
        let user_store_path = std::env::temp_dir().join(format!(
            "destack-runtime-tests/{process_id}/crypto/user-store.keys"
        ));

        // deterministic random options
        let mut options = RuntimeOptions {
            execution: ExecutionMode::Fast,
            random: RandomOptions {
                mode: RandomMode::Deterministic,
                seed: Some(0),
                ..RandomOptions::default()
            },
            ..RuntimeOptions::default()
        };
        options.crypto.host_store_paths.user = Some(user_store_path);

        options
    }

    /// Build a test runtime from explicit runtime options.
    fn from_runtime_options(options: RuntimeOptions) -> Self {
        // build runtime state from explicit options
        let world = World::from_options(&options).expect("runtime test world should build");

        // agent execution isolate
        let agent_tree = NodeTree::new();
        let agent_strings = LocalStringPool::new().into_immutable();
        let agent_engine =
            vm::Isolate::build(agent_tree, agent_strings).expect("agent engine should build");

        let agent = Agent::new_in_world(Vec::new(), &options, &world, Box::new(agent_engine))
            .expect("runtime test agent should build");
        let host = HostSession::from_runtime_options(&options, agent.runtime_id);

        // vm binding isolate
        let tree = NodeTree::new();
        let strings = LocalStringPool::new().into_immutable();
        let vm_isolate = vm::Isolate::build(tree, strings).expect("test vm isolate should build");
        let vm_heap = vm::Heap::default();
        let vm_shared = vm::SharedSpace::default();

        Self {
            world,
            agent: Box::new(agent),
            host,
            vm_isolate: std::cell::RefCell::new(vm_isolate),
            vm_heap: std::cell::RefCell::new(vm_heap),
            vm_shared: std::cell::RefCell::new(vm_shared),
        }
    }

    /// Install default VM bindings using the test agent.
    #[cfg(test)]
    pub(crate) fn install_vm_defaults(&mut self, isolate: &mut vm::Isolate) {
        self.agent.bindings.install_vm_defaults(isolate);
    }

    /// Execute a native binding within a runtime call context.
    pub(crate) fn with_native_call_context<T>(
        &self,
        run: impl FnOnce(&BindingCallContext) -> T,
    ) -> T {
        // install current agent context for vm callback bridges
        let runtime = self.agent.as_ref() as *const Agent;
        let event_loop = self.agent.event_loop.as_ref() as *const _;
        let host = &self.host as *const HostSession;
        let world = self.world.as_ref() as *const World;
        let _agent_guard = enter_current_agent_context(
            runtime,
            event_loop,
            host,
            world,
            self.host.is_process_main_context(),
        );

        // enter a native call context for the binding
        let call_context = BindingCallContext::new(
            &self.agent,
            self.agent.event_loop.as_ref(),
            &self.host,
            &self.world,
        );
        let _guard = enter_binding_call_context(&call_context);

        // run the native call
        run(&call_context)
    }

    /// Drain queued host events for deterministic test setup.
    pub(crate) fn drain_host_events(&self) {
        // clear bootstrap host events before targeted assertions begin
        self.host
            .poll_events(Some(0))
            .expect("host events should drain");
    }

    /// Execute a VM binding within a runtime call context.
    pub(crate) fn with_vm_call_context<T>(
        &self,
        run: impl for<'ctx> FnOnce(&BindingCallContext, &mut vm::ExternalCallContext<'ctx>) -> T,
    ) -> T {
        // install current agent context for vm callback bridges
        let runtime = self.agent.as_ref() as *const Agent;
        let event_loop = self.agent.event_loop.as_ref() as *const _;
        let host = &self.host as *const HostSession;
        let world = self.world.as_ref() as *const World;
        let _agent_guard = enter_current_agent_context(
            runtime,
            event_loop,
            host,
            world,
            self.host.is_process_main_context(),
        );

        // run the VM call with a fresh runtime call context
        let mut isolate = self.vm_isolate.borrow_mut();
        let mut heap = self.vm_heap.borrow_mut();
        let mut shared = self.vm_shared.borrow_mut();
        let mut memory = vm::MemoryContext::new(&mut heap, &mut shared);
        isolate.with_runtime_context(&mut memory, |context| {
            let call_context = BindingCallContext::new(
                &self.agent,
                self.agent.event_loop.as_ref(),
                &self.host,
                &self.world,
            );
            let _guard = enter_binding_call_context(&call_context);
            run(&call_context, context)
        })
    }

    /// Execute the native random binding with deterministic runtime state.
    #[cfg(test)]
    pub(crate) fn call_native_next_u64(&self) -> RuntimeResult<u64> {
        self.with_native_call_context(|_| {
            let mut out = 0u64;
            let status = unsafe { destack_random_stream_next_u64(&mut out) };
            self.status_value(status, "random stream nextU64", out)
        })
    }

    /// Execute the native stream binding with deterministic runtime state.
    #[cfg(test)]
    pub(crate) fn call_native_next_u64_from(&self, stream: u64) -> RuntimeResult<u64> {
        self.with_native_call_context(|_| {
            let mut out = 0u64;
            let status =
                unsafe { destack_random_stream_next_u64_from(&mut out, RandomStream(stream)) };
            self.status_value(status, "random stream nextU64From", out)
        })
    }

    /// Convert a native status and value into a runtime result.
    #[cfg(test)]
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
        let error_id = DiagnosticId::from_raw(status.error_id);
        let error = self
            .agent
            .diagnostics
            .take_error(error_id)
            .unwrap_or_else(|| {
                RuntimeError::from(PlatformError::io(format!(
                    "{label} failed with missing runtime error",
                )))
                .boxed()
            });

        Err(error)
    }

    /// Read the port assigned to a listener handle.
    #[cfg(unix)]
    #[cfg(test)]
    pub(crate) fn listener_port(&self, handle: ListenerHandle) -> u16 {
        let fd = self
            .agent
            .resources
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

    /// Return whether one socket handle is in nonblocking mode.
    #[cfg(unix)]
    #[cfg(test)]
    pub(crate) fn socket_is_nonblocking(&self, handle: SocketHandle) -> bool {
        let fd = self
            .agent
            .resources
            .with_entry(handle.0, |entry| {
                if entry.kind != ResourceKind::Socket {
                    return None;
                }
                entry.fd()
            })
            .flatten()
            .expect("socket handle must be valid");

        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        assert!(flags >= 0, "fcntl(F_GETFL) failed");

        (flags & libc::O_NONBLOCK) != 0
    }

    /// Return whether one socket handle is close-on-exec.
    #[cfg(unix)]
    #[cfg(test)]
    pub(crate) fn socket_is_close_on_exec(&self, handle: SocketHandle) -> bool {
        let fd = self
            .agent
            .resources
            .with_entry(handle.0, |entry| {
                if entry.kind != ResourceKind::Socket {
                    return None;
                }
                entry.fd()
            })
            .flatten()
            .expect("socket handle must be valid");

        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert!(flags >= 0, "fcntl(F_GETFD) failed");

        (flags & libc::FD_CLOEXEC) != 0
    }

    /// Read the port assigned to a listener handle.
    #[cfg(windows)]
    pub(crate) fn listener_port(&self, handle: ListenerHandle) -> u16 {
        use windows_sys::Win32::Networking::WinSock::{
            AF_INET, AF_INET6, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_STORAGE, SOCKET,
            getsockname,
        };

        let socket = self
            .agent
            .resources
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

    /// Return whether one socket handle is close-on-exec or non-inheritable.
    #[cfg(windows)]
    #[cfg(test)]
    pub(crate) fn socket_is_close_on_exec(&self, handle: SocketHandle) -> bool {
        use windows_sys::Win32::Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT};
        use windows_sys::Win32::Networking::WinSock::SOCKET;

        let socket = self
            .agent
            .resources
            .with_entry(handle.0, |entry| {
                if entry.kind != ResourceKind::Socket {
                    return None;
                }
                entry.socket()
            })
            .flatten()
            .expect("socket handle must be valid") as SOCKET;

        let mut flags = 0u32;
        let rc = unsafe { GetHandleInformation(socket as isize, &mut flags) };
        assert_ne!(rc, 0, "GetHandleInformation failed");

        (flags & HANDLE_FLAG_INHERIT) == 0
    }
}
