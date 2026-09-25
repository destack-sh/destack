use std::sync::Arc;

use tspp_heap::{
    AllocationCache, AllocationPlan, Heap, HeapLimits, HeapOptions, HeapReference, RootSlot,
    SharedHeap, SharedHeapLimits, SharedHeapOptions, SharedMarkWorker,
};
use tspp_memory::{MemoryMap, MemoryRange};
use tspp_program as program;
use tspp_program::{FunctionId, Program, StopReason, StopSet, WatchSet, Word};

use crate::diagnostic::ExecutionError;
use crate::machine::Activation;
use crate::{Fiber, Machine, MachineLimits, Result};

use super::{RuntimeCall, TestBinding, TestProgram, TestRuntime};

const MEMORY_BYTES: usize = 512 * 1024 * 1024;
const MEMORY_FRAME_BYTES: usize = 16 * 1024 * 1024;
/// Logical fiber identity mounted by fresh test executions.
pub(crate) const TEST_FIBER_ID: program::FiberId = program::FiberId::new(u32::MAX, 1);

/// One bytecode machine fixture backed by a linked Program.
pub(crate) struct TestMachine {
    /// The linked Program under test.
    program: Arc<Program>,
    /// World memory used by runtime storage and the machine.
    memory: Arc<MemoryMap>,
    /// The fiber carrying test executions.
    fiber: Fiber,
    /// The bytecode machine under test.
    machine: Machine,
    /// Runtime allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[AllocationPlan]>,
    /// Runtime operations observed by bytecode instructions.
    runtime: TestRuntime,
    /// Current dynamically scoped execution context.
    context: program::Context,
    /// Worker heap used by allocation instructions.
    local_heap: Heap,
    /// Runtime heap used by shared allocation instructions.
    shared_heap: SharedHeap,
    /// Worker-local shared allocation cache.
    shared_cache: AllocationCache,
    /// Shared collector worker state.
    shared_mark_worker: SharedMarkWorker,
    /// Immutable constant memory.
    constants: program::StaticSpace,
    /// Worker-local static memory.
    local_statics: program::StaticSpace,
    /// Runtime-shared static memory.
    shared_statics: program::StaticSpace,
    /// The request word polled at safepoints.
    handshake: program::Handshake,
}

#[allow(clippy::too_many_arguments)]
impl TestMachine {
    /// Parse and link one self-contained bytecode module.
    pub(crate) fn parse(source: &str, test: TestProgram) -> Self {
        let program = test.build(source);
        let memory = Arc::new(
            MemoryMap::reserve(MEMORY_BYTES, MEMORY_FRAME_BYTES)
                .expect("test memory should reserve"),
        );

        // build the real runtime storage consumed by one activation
        let mut local_heap = Heap::new(memory.clone(), HeapLimits::default(), HeapOptions::local())
            .expect("test heap should build");
        let mut shared_heap = SharedHeap::new(
            memory.clone(),
            SharedHeapLimits::default(),
            SharedHeapOptions::default(),
        )
        .expect("test shared heap should build");
        let shared_cache = shared_heap.allocation_cache();
        let shared_mark_worker = shared_heap.register_mark_worker();
        let (constants, shared_statics) = program
            .materialize_runtime_statics(memory.clone())
            .expect("test runtime statics should materialize");
        let constant_range = MemoryRange {
            offset: constants.offset(),
            byte_len: constants.byte_len(),
        };
        local_heap.set_constant_range(constant_range);
        shared_heap.set_constant_range(constant_range);
        let local_statics = program
            .materialize_local_statics(memory.clone(), &constants, &shared_statics)
            .expect("test local statics should materialize");
        let allocation_plans = program
            .plan_allocations(local_heap.options(), shared_heap.options())
            .expect("test allocation plans should build")
            .into();
        let machine = Machine::new(program.clone(), MachineLimits::test())
            .expect("test machine should build");
        let fiber = machine
            .reserve_fiber(memory.clone())
            .expect("test fiber should reserve");

        Self {
            program,
            memory,
            fiber,
            machine,
            allocation_plans,
            runtime: TestRuntime::default(),
            context: program::Context::empty(),
            local_heap,
            shared_heap,
            shared_cache,
            shared_mark_worker,
            constants,
            local_statics,
            shared_statics,
            handshake: program::Handshake::new(),
        }
    }

    /// Execute one function and require normal completion.
    pub(crate) fn complete(&mut self, function: u32, arguments: &[Word]) -> Vec<Word> {
        let outcome = self
            .run(function, arguments, None, None, None)
            .unwrap_or_else(|error| panic!("f{function} should execute: {error}"));

        Self::completion("function", outcome)
    }

    /// Execute one profiled function and require normal completion.
    pub(crate) fn complete_profiled(
        &mut self,
        function: u32,
        arguments: &[Word],
        profile: &mut program::Profile,
    ) -> Vec<Word> {
        let outcome = self
            .run(function, arguments, None, None, Some(profile))
            .unwrap_or_else(|error| panic!("f{function} should execute: {error}"));

        Self::completion("function", outcome)
    }

    /// Execute one function and return its raw machine outcome.
    pub(crate) fn run(
        &mut self,
        function: u32,
        arguments: &[Word],
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        profile: Option<&mut program::Profile>,
    ) -> Result<program::Outcome<Vec<Word>>> {
        let function = FunctionId(function);

        // mount the test scheduler's root identity on fresh execution
        if self.fiber.is_idle() {
            self.fiber.mount(TEST_FIBER_ID);
        }

        self.activation(stop_points, watch_points, profile, None)
            .run(function, arguments)
            .map_err(ExecutionError::into_error)
    }

    /// Return and clear exact runtime boundary calls.
    pub(crate) fn take_runtime_calls(&mut self) -> Vec<RuntimeCall> {
        self.runtime.take_calls()
    }

    /// Select Program execution event categories.
    pub(crate) fn select_events(&mut self, kinds: impl IntoIterator<Item = program::EventKind>) {
        self.runtime.select_events(kinds);
    }

    /// Return and clear exact Program execution events.
    pub(crate) fn take_events(&mut self) -> Vec<program::Event> {
        self.runtime.take_events()
    }

    /// Request one runtime poll action.
    pub(crate) fn request_poll(&mut self, action: program::Poll) {
        self.runtime.request_poll(action);
    }

    /// Execute one function and require one debugger stop.
    pub(crate) fn run_to_stop(
        &mut self,
        function: u32,
        arguments: &[Word],
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
    ) -> StopReason {
        let outcome = self
            .run(function, arguments, stop_points, watch_points, None)
            .unwrap_or_else(|error| panic!("f{function} should execute: {error}"));
        let program::Outcome::Stopped { reason } = outcome else {
            panic!("f{function} should stop");
        };

        reason
    }

    /// Continue one stopped execution.
    pub(crate) fn continue_execution(
        &mut self,
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        profile: Option<&mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> Result<program::Outcome<Vec<Word>>> {
        self.activation(stop_points, watch_points, profile, resume_skip)
            .execute()
            .map_err(ExecutionError::into_error)
    }

    /// Continue one stopped execution and require normal completion.
    pub(crate) fn continue_to_completion(
        &mut self,
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> Vec<Word> {
        let outcome = self
            .continue_execution(stop_points, watch_points, None, resume_skip)
            .unwrap_or_else(|error| panic!("stopped execution should continue: {error}"));

        Self::completion("stopped execution", outcome)
    }

    /// Fork the stopped fiber into one forked memory map.
    pub(crate) fn fork_fiber(&self) -> (crate::Fiber, Arc<MemoryMap>) {
        let memory = Arc::new(
            self.memory
                .fork_lazy()
                .expect("test machine memory should fork"),
        );
        let fiber = self.fiber.fork(memory.clone());

        (fiber, memory)
    }

    /// Adopt one forked fiber over its forked memory map.
    pub(crate) fn adopt_fiber(&mut self, fiber: crate::Fiber, memory: Arc<MemoryMap>) {
        self.memory = memory;
        self.fiber = fiber;
    }

    /// Return the worker heap roots retained by the machine.
    pub(crate) fn roots(&mut self) -> Vec<HeapReference> {
        let mut roots = Vec::new();
        self.machine
            .visit_root_slots(&mut self.fiber, &mut |slot| {
                let root = match slot {
                    RootSlot::HeapReference(reference) => *reference,
                    RootSlot::HeapBytes(bytes) => HeapReference::read_from_bytes(bytes)?,
                    RootSlot::SharedHeapBytes(_) | RootSlot::BorrowBytes(_) => return Ok(()),
                };
                if !root.is_nullish() {
                    roots.push(root);
                }

                Ok(())
            })
            .expect("test roots should visit");

        roots
    }

    /// Return one completed value or fail the current test.
    fn completion(operation: &str, outcome: program::Outcome<Vec<Word>>) -> Vec<Word> {
        let program::Outcome::Completed { value } = outcome else {
            panic!("{operation} should complete");
        };

        value
    }

    /// Bind one clean activation to this test machine.
    fn activation<'machine, 'run>(
        &'machine mut self,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> Activation<'machine, 'run, TestRuntime>
    where
        'machine: 'run,
    {
        let activation = program::Activation {
            runtime: &mut self.runtime,
            context: &mut self.context,
            memory: program::Memory {
                allocation_plans: &self.allocation_plans,
                local_heap: &mut self.local_heap,
                shared_heap: &self.shared_heap,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_statics: &mut self.local_statics,
                shared_statics: &mut self.shared_statics,
                constants: &self.constants,
                handshake: &self.handshake,
            },
        };

        Activation::new(&mut self.machine, &mut self.fiber, activation).instrument(
            stop_points,
            watch_points,
            profile,
            resume_skip,
        )
    }

    /// Return the linked Program.
    pub(crate) fn program(&self) -> &Program {
        &self.program
    }

    /// Register one runtime binding implementation.
    pub(crate) fn bind(&mut self, name: &'static str, binding: TestBinding) {
        self.runtime.bind(name, binding);
    }
}
