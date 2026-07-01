use std::sync::Arc;

use destack_compiler::ProgramLinker;
use destack_heap::{
    AllocationCache, Allocator, Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits,
    SharedHeapOptions, SharedMarkWorker,
};
use destack_mir as mir;
use destack_program::{FunctionId, StaticSpace, Value};
use destack_source::{DiagnosticSeverity, FileId, PackageId, Uri};
use destack_vm::{Machine, MachineOptions};
use mir::parse::{ParseOptions, Parser};

/// Runtime state needed to call one benchmark entry.
pub(crate) struct Runtime {
    /// The machine under measurement.
    machine: Machine,
    /// The worker-local static byte space.
    local_static: StaticSpace,
    /// The shared static byte space.
    shared_static: StaticSpace,
    /// The worker heap.
    pub(crate) heap: Heap,
    /// The runtime shared heap.
    shared: SharedHeap,
    /// The worker-local shared allocation cache.
    shared_cache: AllocationCache,
    /// The shared mark worker.
    shared_mark_worker: SharedMarkWorker,
    /// The benchmark entry function.
    entry: FunctionId,
}

impl Runtime {
    /// Build one benchmark runtime from MIR text and entry name.
    pub(crate) fn new(program: &str, entry: &str) -> Self {
        // parse the benchmark program
        let file_id = FileId::from_source_bytes(program.as_bytes());
        let parsed = Parser::parse(file_id, program, ParseOptions::default());
        let (tree, target_layout, types, layouts, dispatch, _, _, _, _, strings, diagnostics) =
            parsed.into_parts();
        if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            panic!("benchmark MIR should parse");
        }

        // build runtime memory
        let options = MachineOptions::unbounded();
        let mut local_static = StaticSpace::empty();
        let mut shared_static = StaticSpace::empty();
        let heap = heap();
        let shared = shared_heap();
        let shared_mark_worker = shared.register_mark_worker();
        let shared_cache = shared.allocation_cache();

        // build the VM machine
        let program = ProgramLinker::new(
            benchmark_package_id(),
            tree,
            target_layout,
            types,
            layouts,
            dispatch,
            strings,
            options.heap.clone(),
            options.shared_heap.clone(),
        )
        .build()
        .expect("benchmark program should link");
        let mut machine =
            Machine::new(Arc::new(program), options).expect("benchmark machine should build");

        // initialize program statics
        machine
            .initialize(&heap, &shared, &mut local_static, &mut shared_static)
            .expect("benchmark machine should initialize");

        // resolve the entry once
        let entry = machine
            .function_id_by_name(entry)
            .expect("benchmark entry should exist");

        Self {
            machine,
            local_static,
            shared_static,
            heap,
            shared,
            shared_cache,
            shared_mark_worker,
            entry,
        }
    }

    /// Run the benchmark entry once.
    pub(crate) fn run(&mut self, iterations: i32) -> Value {
        let arguments = [Value::int32(iterations)];

        self.run_with_arguments(self.entry, &arguments)
    }

    /// Run one benchmark entry with explicit arguments.
    pub(crate) fn run_with_arguments(&mut self, entry: FunctionId, arguments: &[Value]) -> Value {
        self.machine
            .run_function(
                &mut self.local_static,
                &mut self.shared_static,
                &mut self.heap,
                &self.shared,
                &mut self.shared_cache,
                &self.shared_mark_worker,
                entry,
                arguments,
            )
            .expect("benchmark function should run")
    }

    /// Return one benchmark entry by function name.
    pub(crate) fn entry(&self, name: &str) -> FunctionId {
        self.machine
            .function_id_by_name(name)
            .expect("benchmark entry should exist")
    }

    /// Fork the benchmark heap with the machine trace table.
    pub(crate) fn fork_heap(&mut self) -> Heap {
        let trace_maps = self.machine.trace_maps();

        self.heap
            .fork(trace_maps)
            .expect("benchmark heap should fork")
    }
}

/// Return the package id used by VM dispatch benchmarks.
fn benchmark_package_id() -> PackageId {
    PackageId::from_uri(&Uri::logical("bench/vm-dispatch"))
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
