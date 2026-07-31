use std::sync::Arc;

use destack_heap::{
    AllocationCache, AllocationPlan, Heap, HeapLimits, HeapOptions, Root, SharedHeap,
    SharedHeapLimits, SharedHeapOptions, SharedMarkWorker,
};
use destack_memory::MemoryMap;
use destack_program as program;
use destack_program::{Completion, FunctionId, Program, StopReason, StopSet, WatchSet, Word};

use crate::diagnostic::ExecutionError;
use crate::machine::{Activation, Return};
use crate::{Machine, MachineLimits, Result};

use super::{RuntimeCall, TestBinding, TestProgram, TestRuntime};

const MEMORY_BYTES: usize = 512 * 1024 * 1024;
const MEMORY_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// One bytecode machine fixture backed by a linked Program.
pub(crate) struct TestMachine {
    /// The linked Program under test.
    program: Arc<Program>,
    /// World memory used by runtime storage and the machine.
    memory: Arc<MemoryMap>,
    /// Worker-local continuation storage.
    continuations: program::ContinuationTable,
    /// The bytecode machine under test.
    machine: Machine,
    /// Runtime allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[Option<AllocationPlan>]>,
    /// Runtime operations observed by bytecode instructions.
    runtime: TestRuntime,
    /// Worker heap used by allocation instructions.
    heap: Heap,
    /// Runtime heap used by shared allocation instructions.
    shared_heap: SharedHeap,
    /// Worker-local shared allocation cache.
    shared_cache: AllocationCache,
    /// Shared collector worker state.
    shared_mark_worker: SharedMarkWorker,
    /// Worker-local static memory.
    local_static: program::StaticSpace,
    /// Runtime-shared static memory.
    shared_static: program::StaticSpace,
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
        let heap = Heap::new(memory.clone(), HeapLimits::default(), HeapOptions::local())
            .expect("test heap should build");
        let shared_heap = SharedHeap::new(
            memory.clone(),
            SharedHeapLimits::default(),
            SharedHeapOptions::default(),
        )
        .expect("test shared heap should build");
        let shared_cache = shared_heap.allocation_cache();
        let shared_mark_worker = shared_heap.register_mark_worker();
        let local_static = program
            .materialize_local_statics(memory.clone())
            .expect("test local statics should materialize");
        let shared_static = program
            .materialize_shared_statics(memory.clone())
            .expect("test shared statics should materialize");
        let allocation_plans = program
            .plan_allocations(heap.options(), shared_heap.options())
            .expect("test allocation plans should build")
            .into();
        let machine = Machine::new(program.clone(), memory.clone(), MachineLimits::test())
            .expect("test machine should build");

        Self {
            program,
            memory,
            continuations: program::ContinuationTable::default(),
            machine,
            allocation_plans,
            runtime: TestRuntime::default(),
            heap,
            shared_heap,
            shared_cache,
            shared_mark_worker,
            local_static,
            shared_static,
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

        self.activation(stop_points, watch_points, profile, None)
            .run(function, arguments)
            .map_err(ExecutionError::into_error)
    }

    /// Resume one suspended coroutine and return its raw machine outcome.
    pub(crate) fn resume(
        &mut self,
        continuation: program::Continuation,
        values: &[Word],
    ) -> Result<program::Outcome<Vec<Word>>> {
        let completion = continuation.completion();
        self.enter_continuation(continuation, completion)?;

        self.activation(None, None, None, None)
            .resume(values)
            .map_err(ExecutionError::into_error)
    }

    /// Complete one suspended generator and return its raw machine outcome.
    pub(crate) fn complete_continuation(
        &mut self,
        continuation: program::Continuation,
        values: &[Word],
    ) -> Result<program::Outcome<Vec<Word>>> {
        let completion = continuation.completion();
        self.enter_continuation(continuation, completion)?;

        self.activation(None, None, None, None)
            .complete(values)
            .map_err(ExecutionError::into_error)
    }

    /// Cancel one suspended asynchronous continuation.
    pub(crate) fn cancel(
        &mut self,
        continuation: program::Continuation,
    ) -> Result<program::Outcome<Vec<Word>>> {
        self.enter_continuation(continuation, Completion::Cancel)?;

        self.activation(None, None, None, None)
            .cancel()
            .map_err(ExecutionError::into_error)
    }

    /// Execute one function and require asynchronous suspension.
    pub(crate) fn run_to_await(
        &mut self,
        function: u32,
        arguments: &[Word],
    ) -> (FunctionId, Vec<Word>, program::Continuation) {
        let outcome = self
            .run(function, arguments, None, None, None)
            .unwrap_or_else(|error| panic!("f{function} should execute: {error}"));
        let program::Outcome::Awaited {
            park,
            awaitable,
            continuation,
        } = outcome
        else {
            panic!("f{function} should await");
        };

        (park, awaitable, continuation)
    }

    /// Execute one function and require generator suspension.
    pub(crate) fn run_to_yield(
        &mut self,
        function: u32,
        arguments: &[Word],
    ) -> (Vec<Word>, program::Continuation) {
        let outcome = self
            .run(function, arguments, None, None, None)
            .unwrap_or_else(|error| panic!("f{function} should execute: {error}"));
        let program::Outcome::Yielded {
            value,
            continuation,
        } = outcome
        else {
            panic!("f{function} should yield");
        };

        (value, continuation)
    }

    /// Return the canonical bytes retained by one continuation.
    pub(crate) fn continuation_bytes(&self, continuation: &program::Continuation) -> Vec<u8> {
        let range = continuation.memory();

        self.memory
            .read_bytes(range.offset, range.byte_len)
            .expect("continuation bytes should remain readable")
    }

    /// Duplicate one continuation inside this test machine.
    pub(crate) fn fork_continuation(
        &self,
        continuation: &program::Continuation,
    ) -> program::Continuation {
        continuation
            .fork(&self.memory)
            .expect("continuation should fork")
    }

    /// Resume one continuation and require normal completion.
    pub(crate) fn resume_to_completion(
        &mut self,
        continuation: program::Continuation,
        values: &[Word],
    ) -> Vec<Word> {
        let outcome = self
            .resume(continuation, values)
            .unwrap_or_else(|error| panic!("continuation should resume: {error}"));

        Self::completion("resumed continuation", outcome)
    }

    /// Complete one continuation and require normal completion.
    pub(crate) fn complete_to_completion(
        &mut self,
        continuation: program::Continuation,
        values: &[Word],
    ) -> Vec<Word> {
        let outcome = self
            .complete_continuation(continuation, values)
            .unwrap_or_else(|error| panic!("continuation should complete: {error}"));

        Self::completion("completed continuation", outcome)
    }

    /// Return and clear exact runtime boundary calls.
    pub(crate) fn take_runtime_calls(&mut self) -> Vec<RuntimeCall> {
        self.runtime.take_calls()
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
        self.machine.materialize()?;

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

    /// Copy the captured activation into one forked memory map.
    pub(crate) fn capture(&self) -> (program::ActivationImage, Arc<MemoryMap>) {
        let memory = Arc::new(
            self.memory
                .fork_lazy()
                .expect("test machine memory should fork"),
        );
        let mut machine = self.machine.fork(memory.clone());
        let image = machine
            .take_activation()
            .expect("test machine should contain one captured activation");

        (image, memory)
    }

    /// Restore one canonical activation over its captured memory image.
    pub(crate) fn restore(&mut self, image: program::ActivationImage, memory: Arc<MemoryMap>) {
        let mut machine = Machine::new(self.program.clone(), memory.clone(), MachineLimits::test())
            .expect("test machine should build");
        machine.restore(image).expect("test machine should restore");
        self.memory = memory;
        self.machine = machine;
    }

    /// Return heap roots retained by the machine.
    pub(crate) fn roots(&mut self) -> Vec<Root> {
        let mut roots = Vec::new();
        self.machine
            .visit_root_slots(&mut |slot| {
                roots.push(slot.load()?);

                Ok(())
            })
            .expect("test roots should visit");
        self.continuations
            .visit_root_slots(&self.program, &self.memory, &mut |slot| {
                roots.push(slot.load()?);

                Ok(())
            })
            .expect("test continuation roots should visit");

        roots
    }

    /// Return one completed value or fail the current test.
    fn completion(operation: &str, outcome: program::Outcome<Vec<Word>>) -> Vec<Word> {
        let program::Outcome::Completed { value } = outcome else {
            panic!("{operation} should complete");
        };

        value
    }

    /// Enter one continuation through the raw bytecode test boundary.
    fn enter_continuation(
        &mut self,
        continuation: program::Continuation,
        completion: Completion,
    ) -> Result<()> {
        let return_to = Return::Exit { completion };
        self.machine
            .consume_continuation(continuation, |machine, continuation| {
                machine.materialize_continuation(continuation, return_to)
            })
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
            memory: program::Memory {
                allocation_plans: &self.allocation_plans,
                heap: &mut self.heap,
                shared_heap: &self.shared_heap,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static: &mut self.shared_static,
                constant_space: self.program.constants(),
            },
        };

        Activation::new(&mut self.machine, &mut self.continuations, activation).instrument(
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

    /// Build one exact Program value for runtime call assertions.
    pub(crate) fn value(
        &self,
        ty: program::TypeId,
        words: impl IntoIterator<Item = Word>,
    ) -> program::Value {
        self.program
            .value(ty, words)
            .expect("test value should match linked Program metadata")
    }
}
