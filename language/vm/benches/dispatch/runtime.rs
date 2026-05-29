use std::sync::Arc;

use destack_engine::{EngineId, StaticSpace, Value};
use destack_heap::{
    AllocationCache, Allocator, GcWorker, Heap, HeapLimits, HeapOptions, SharedHeap,
    SharedHeapLimits, SharedHeapOptions,
};
use destack_mir as mir;
use destack_source::FileId;
use destack_vm::{Isolate, IsolateOptions};
use mir::parse::{ParseOptions, Parser};

/// Runtime state needed to call one benchmark entry.
pub(crate) struct Runtime {
    /// The isolate under measurement.
    isolate: Isolate,
    /// The worker static byte space.
    statics: StaticSpace,
    /// The worker heap.
    pub(crate) heap: Heap,
    /// The runtime shared heap.
    shared: SharedHeap,
    /// The worker-local shared allocation cache.
    shared_cache: AllocationCache,
    /// The shared collector worker.
    shared_gc: GcWorker,
    /// The benchmark entry function.
    entry: mir::LocalNodeId<mir::Function>,
}

impl Runtime {
    /// Build one benchmark runtime from MIR text and entry name.
    pub(crate) fn new(program: &str, entry: &str) -> Self {
        // parse the benchmark program
        let (tree, strings) = Parser::parse(FileId::new(0), program, ParseOptions::default())
            .finish()
            .expect("benchmark MIR should parse");

        // build the VM isolate
        let mut isolate = Isolate::build_with_options(
            EngineId::new(1),
            tree,
            strings,
            IsolateOptions::unbounded(),
        )
        .expect("benchmark isolate should build");

        // build runtime memory
        let mut statics = StaticSpace::empty();
        let heap = heap();
        let shared = shared_heap();
        let shared_gc = shared.register_collector_worker();
        let shared_cache = shared.allocation_cache();

        // initialize program statics
        isolate
            .initialize(&heap, &shared, &mut statics)
            .expect("benchmark isolate should initialize");

        // resolve the entry once
        let entry = isolate
            .function_id_by_name(entry)
            .expect("benchmark entry should exist");

        Self {
            isolate,
            statics,
            heap,
            shared,
            shared_cache,
            shared_gc,
            entry,
        }
    }

    /// Run the benchmark entry once.
    pub(crate) fn run(&mut self, iterations: i32) -> Value {
        let arguments = [Value::int32(iterations)];

        self.run_with_arguments(self.entry, &arguments)
    }

    /// Run one benchmark entry with explicit arguments.
    pub(crate) fn run_with_arguments(
        &mut self,
        entry: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> Value {
        self.isolate
            .run_function(
                &mut self.statics,
                &mut self.heap,
                &self.shared,
                &mut self.shared_cache,
                &self.shared_gc,
                entry,
                arguments,
            )
            .expect("benchmark function should run")
    }

    /// Return one benchmark entry by function name.
    pub(crate) fn entry(&self, name: &str) -> mir::LocalNodeId<mir::Function> {
        self.isolate
            .function_id_by_name(name)
            .expect("benchmark entry should exist")
    }

    /// Fork the benchmark heap with the isolate trace table.
    pub(crate) fn fork_heap(&mut self) -> Heap {
        let trace_table = self.isolate.trace_table();

        self.heap
            .fork(trace_table.as_ref())
            .expect("benchmark heap should fork")
    }
}

/// Create one worker heap for benchmark execution.
fn heap() -> Heap {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
            .expect("benchmark allocator should build"),
    );

    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("benchmark heap should build")
}

/// Create one shared heap for benchmark execution.
fn shared_heap() -> SharedHeap {
    let options = SharedHeapOptions::default();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
            .expect("benchmark shared page allocator should build"),
    );

    SharedHeap::with_allocator_limits_and_options(allocator, SharedHeapLimits::default(), options)
        .expect("benchmark shared heap should build")
}
