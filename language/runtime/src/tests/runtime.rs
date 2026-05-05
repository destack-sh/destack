use destack_core::{ImmutableStringPool, LocalStringPool};
use destack_mir::Tree;
use destack_mir::parse::{ParseOptions, Parser};
use destack_source::FileId;
use destack_workspace::{ExecutionMode, RandomMode, RuntimeOptions};
use {destack_engine as engine, destack_vm as vm};

#[cfg(test)]
use crate::diagnostic::{DiagnosticId, RuntimeError, RuntimeResult, RuntimeStatus};
use crate::host::Session;
#[cfg(test)]
use crate::platform::PlatformError;
#[cfg(test)]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(test)]
use crate::platform::random::{
    RandomStream, destack_random_stream_next_u64, destack_random_stream_next_u64_from,
};
use crate::runtime::bindings::BindingEngine;
use crate::runtime::{
    BindingCallContext, RuntimeSharedHeap, Worker, World, WorldScope, enter_binding_call_context,
    enter_current_worker_context,
};

/// The canonical string type fixture used by runtime VM tests.
const STRING_TYPE_ALIAS: &str = "type String {\n    lengthUtf16: uint32;\n    lengthBytes: uint32;\n    data: slice<uint8, readonly>;\n}\n";

/// Runtime harness for runtime tests.
pub(crate) struct TestRuntime {
    /// World that owns the worker lifetime.
    world: World,
    /// Worker under test.
    pub worker: Box<Worker>,
    /// Runtime-owned shared heap state retained for the worker lifetime.
    _shared: RuntimeSharedHeap,
    /// Host under test.
    host: Session,
    /// VM isolate backing VM bindings in tests.
    vm_isolate: std::cell::RefCell<vm::Isolate>,
    /// Heap backing the VM isolate in tests.
    vm_heap: std::cell::RefCell<vm::Heap>,
    /// Shared memory backing the VM isolate in tests.
    vm_shared: std::cell::RefCell<vm::SharedHeap>,
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

    /// Build a runtime with deterministic random settings and no ambient host ingress.
    pub(crate) fn deterministic_random_without_host_ambient_ingress() -> Self {
        Self::deterministic_random_without_host_ambient_ingress_with_options(|_| {})
    }

    /// Build a runtime with deterministic random settings, one options mutator, and no ambient host ingress.
    pub(crate) fn deterministic_random_without_host_ambient_ingress_with_options(
        configure: impl FnOnce(&mut RuntimeOptions),
    ) -> Self {
        // build baseline deterministic options
        let mut options = Self::deterministic_runtime_options();
        configure(&mut options);

        // runtime with deterministic random state and no ambient ingress
        Self::from_runtime_options_with_native_ingress(options, false)
    }

    /// Build the baseline deterministic runtime options.
    fn deterministic_runtime_options() -> RuntimeOptions {
        // route host-backed crypto snapshots to deterministic local files
        let process_id = std::process::id();
        let user_store_path = std::env::temp_dir().join(format!(
            "destack-runtime-tests/{process_id}/crypto/user-store.keys"
        ));

        // deterministic random options
        let mut options = RuntimeOptions::default();
        options.set_execution_mode(ExecutionMode::Fast);
        options.set_random_mode(RandomMode::Deterministic);
        options.simulation.random.seed = Some(0);
        options.crypto.host_store_paths.user = Some(user_store_path);

        options
    }

    /// Build a test runtime from explicit runtime options.
    fn from_runtime_options(options: RuntimeOptions) -> Self {
        Self::from_runtime_options_with_native_ingress(options, true)
    }

    /// Build a test runtime from explicit runtime options and one native-ingress policy.
    fn from_runtime_options_with_native_ingress(
        options: RuntimeOptions,
        is_native_ingress_enabled: bool,
    ) -> Self {
        // build runtime state from explicit options
        let mut world = World::from_options(&options).expect("runtime test world should build");

        // worker execution isolate
        let agent_tree = Tree::new();
        let agent_strings = LocalStringPool::new().into_immutable();
        let agent_engine = vm::Isolate::build(vm::IsolateId::new(1), agent_tree, agent_strings)
            .expect("worker engine should build");

        let world_scope = world.world_scope();
        let lineage = world.lineage.read();
        let shared = RuntimeSharedHeap::new(lineage.allocator(), lineage.collector(), &options)
            .expect("runtime shared heap should build");
        drop(lineage);
        let runtime_static = engine::StaticSpace::empty();
        let mut worker = Worker::new_in_world(
            Vec::new(),
            &options,
            &world_scope,
            &shared,
            &runtime_static,
            agent_engine,
        )
        .expect("runtime test worker should build");
        let host = if is_native_ingress_enabled {
            Session::from_runtime_options(&options, worker.runtime_id)
        } else {
            Session::from_runtime_options_with_native_ingress(&options, worker.runtime_id, false)
        };

        // vm binding isolate
        let (tree, strings) = test_vm_isolate_module();
        let mut vm_isolate = vm::Isolate::build(vm::IsolateId::new(2), tree, strings)
            .expect("test vm isolate should build");
        worker
            .bindings
            .install_vm_defaults(&mut vm_isolate)
            .expect("test vm bindings should install");
        let vm_heap = vm::Heap::with_allocator_limits_and_options(
            std::sync::Arc::new(
                vm::Allocator::try_new(
                    vm::HeapOptions::local().page_bytes,
                    vm::HeapOptions::local().allocator_chunk_bytes,
                )
                .expect("test vm allocator should build"),
            ),
            vm::HeapLimits::default(),
            vm::HeapOptions::local(),
        )
        .expect("test vm heap should build");
        let vm_shared = vm::SharedHeap::with_allocator_limits_and_options(
            std::sync::Arc::new(
                vm::Allocator::try_new(
                    vm::HeapOptions::shared().page_bytes,
                    vm::HeapOptions::shared().allocator_chunk_bytes,
                )
                .expect("test shared allocator should build"),
            ),
            vm::SharedHeapLimits::default(),
            vm::HeapOptions::shared(),
        )
        .expect("test vm shared heap should build");

        Self {
            world,
            worker: Box::new(worker),
            _shared: shared,
            host,
            vm_isolate: std::cell::RefCell::new(vm_isolate),
            vm_heap: std::cell::RefCell::new(vm_heap),
            vm_shared: std::cell::RefCell::new(vm_shared),
        }
    }

    /// Install default VM bindings using the test worker.
    #[cfg(test)]
    pub(crate) fn install_vm_defaults(&mut self, isolate: &mut vm::Isolate) {
        self.worker
            .bindings
            .install_vm_defaults(isolate)
            .expect("test vm bindings should install");
    }

    /// Execute a native binding within a runtime call context.
    pub(crate) fn with_native_call_context<T>(
        &mut self,
        run: impl FnOnce(&BindingCallContext) -> T,
    ) -> T {
        let world = self.world_scope();

        // install current worker context for vm callback bridges
        let runtime = self.worker.as_ref() as *const Worker;
        let event_loop = self.worker.event_loop.as_ref() as *const _;
        let host = &self.host as *const Session;
        let world_ptr = &world as *const _;
        let _agent_guard = enter_current_worker_context(
            runtime,
            event_loop,
            host,
            world_ptr,
            self.host.is_process_main_context(),
        );

        // enter a native call context for the binding
        let call_context = BindingCallContext::from_raw(
            self.worker.as_ref() as *const Worker,
            self.worker.event_loop.as_ref() as *const _,
            &self.host as *const Session,
            &world as *const WorldScope,
            BindingEngine::Native,
        );
        let _guard = enter_binding_call_context(&call_context);

        // run the native call
        run(&call_context)
    }

    /// Drain queued host events for deterministic test setup.
    pub(crate) fn drain_host_events(&self) {
        // clear bootstrap host events before targeted assertions begin
        self.host.poll(Some(0)).expect("host events should drain");
    }

    /// Execute a VM binding within a runtime call context.
    pub(crate) fn with_vm_call_context<T>(
        &mut self,
        run: impl for<'ctx> FnOnce(&BindingCallContext, &mut vm::BindingContext<'ctx>) -> T,
    ) -> T {
        let world = self.world_scope();

        // install current worker context for vm callback bridges
        let runtime = self.worker.as_ref() as *const Worker;
        let event_loop = self.worker.event_loop.as_ref() as *const _;
        let host = &self.host as *const Session;
        let world_ptr = &world as *const _;
        let _agent_guard = enter_current_worker_context(
            runtime,
            event_loop,
            host,
            world_ptr,
            self.host.is_process_main_context(),
        );

        // run the VM call with a fresh runtime call context
        let mut isolate = self.vm_isolate.borrow_mut();
        let mut heap = self.vm_heap.borrow_mut();
        let mut shared = self.vm_shared.borrow_mut();
        isolate
            .with_runtime_context(&mut heap, &mut shared, Default::default(), |context| {
                let call_context = BindingCallContext::from_raw(
                    self.worker.as_ref() as *const Worker,
                    self.worker.event_loop.as_ref() as *const _,
                    &self.host as *const Session,
                    &world as *const WorldScope,
                    BindingEngine::Vm,
                );
                let _guard = enter_binding_call_context(&call_context);
                Ok(run(&call_context, context))
            })
            .expect("runtime context should release pins")
    }

    /// Borrow one execution view from the owned test world.
    fn world_scope(&mut self) -> WorldScope {
        self.world.world_scope()
    }

    /// Execute the native random binding with deterministic runtime state.
    #[cfg(test)]
    pub(crate) fn call_native_next_u64(&mut self) -> RuntimeResult<u64> {
        let (status, out) = self.with_native_call_context(|_| {
            let mut out = 0u64;
            let status = unsafe { destack_random_stream_next_u64(&mut out) };
            (status, out)
        });

        self.status_value(status, "random stream nextU64", out)
    }

    /// Execute the native stream binding with deterministic runtime state.
    #[cfg(test)]
    pub(crate) fn call_native_next_u64_from(&mut self, stream: u64) -> RuntimeResult<u64> {
        let (status, out) = self.with_native_call_context(|_| {
            let mut out = 0u64;
            let status =
                unsafe { destack_random_stream_next_u64_from(&mut out, RandomStream(stream)) };
            (status, out)
        });

        self.status_value(status, "random stream nextU64From", out)
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
            .worker
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
}

/// Build the minimal MIR module required for one VM binding test isolate.
fn test_vm_isolate_module() -> (Tree, ImmutableStringPool) {
    let (tree, strings) = Parser::parse(FileId::new(0), STRING_TYPE_ALIAS, ParseOptions::default())
        .validate()
        .expect("runtime vm test isolate should parse");

    (tree, strings)
}

#[cfg(test)]
mod tests {
    use super::TestRuntime;
    use crate::platform::VmSlice;
    use destack_heap::HeapReference;

    /// Empty heap backed slices use one null backing reference.
    #[test]
    fn test_empty_vm_slice_uses_null_heap_backing() {
        let mut runtime = TestRuntime::deterministic_random();

        runtime.with_vm_call_context(|_binding, context| {
            let slice =
                VmSlice::<u32>::from_values(&mut context.write(), &[]).expect("slice should build");

            assert_eq!(slice.len, 0);
            assert_eq!(slice.data.as_heap_reference(), HeapReference::NULL);

            let values = slice
                .read_values(&context.read())
                .expect("slice should read");

            assert!(values.is_empty());
        });
    }
}
