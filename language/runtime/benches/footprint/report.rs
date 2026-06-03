use std::mem::size_of;
use std::sync::Once;

use destack_engine::StaticSpace;
use destack_heap::{
    AllocationCache, Allocator, GcWorker, Heap, HeapOptions, SharedHeap, SharedHeapOptions,
    SizeClassTable,
};
use destack_runtime::diagnostic::DiagnosticStore;
use destack_runtime::host::HostPollResult;
use destack_runtime::host::binding::BindingRegistry;
use destack_runtime::host::resource::ResourceTable;
use destack_runtime::runtime::random::Random;
use destack_runtime::runtime::scheduler::EventLoop;
use destack_runtime::runtime::{Runtime, Worker};
use destack_runtime::world::trace::{Observations, Trace, TraceLog};
use destack_runtime::world::{Entity, Policy, World};
use destack_vm::{Continuation, ContinuationImage, Machine, StackImage};
use destack_workspace::{Environment, RuntimeOptions};

use crate::ALLOCATOR;
use crate::measure::AllocationSample;
use crate::setup::{RuntimeSetup, VmSetup};

static PRINT: Once = Once::new();

/// Print the footprint summary once per benchmark process.
pub(crate) fn print_once() {
    PRINT.call_once(|| {
        let runtime = RuntimeSetup::new();
        let vm = VmSetup::new();

        print_type_sizes();
        print_component_sizes();
        print_allocations(&runtime, vm);
        print_vm_machine_breakdown(vm);
    });
}

/// Print static type sizes.
fn print_type_sizes() {
    let rows = [
        ("world", "World", size_of::<World>()),
        ("world", "Policy", size_of::<Policy>()),
        ("world", "Trace", size_of::<Trace>()),
        ("world", "TraceLog", size_of::<TraceLog>()),
        ("world", "Observations", size_of::<Observations>()),
        ("world", "Entity", size_of::<Entity>()),
        ("runtime", "Runtime", size_of::<Runtime>()),
        ("runtime", "Worker", size_of::<Worker>()),
        ("runtime", "EventLoop", size_of::<EventLoop>()),
        ("runtime", "Random", size_of::<Random>()),
        ("host", "HostPollResult", size_of::<HostPollResult>()),
        ("host", "ResourceTable", size_of::<ResourceTable>()),
        ("host", "BindingRegistry", size_of::<BindingRegistry>()),
        (
            "diagnostic",
            "DiagnosticStore",
            size_of::<DiagnosticStore>(),
        ),
        ("workspace", "Environment", size_of::<Environment>()),
        ("workspace", "RuntimeOptions", size_of::<RuntimeOptions>()),
        ("engine", "StaticSpace", size_of::<StaticSpace>()),
        ("vm", "Machine", size_of::<Machine>()),
        ("vm", "Continuation", size_of::<Continuation>()),
        ("vm", "ContinuationImage", size_of::<ContinuationImage>()),
        ("vm", "StackImage", size_of::<StackImage>()),
        ("heap", "Heap", size_of::<Heap>()),
        ("heap", "SharedHeap", size_of::<SharedHeap>()),
    ];

    eprintln!();
    eprintln!("runtime footprint: static type sizes");
    eprintln!("{:<12} {:<24} {:>12}", "module", "type", "size");
    eprintln!("{:-<12} {:-<24} {:-<12}", "", "", "");
    for (module, name, bytes) in rows {
        eprintln!("{module:<12} {name:<24} {:>12}", format_bytes(bytes as i64));
    }
}

/// Print public component sizes inside the larger owner nouns.
fn print_component_sizes() {
    let rows = [
        ("heap", "Allocator", size_of::<Allocator>()),
        ("heap", "SizeClassTable", size_of::<SizeClassTable>()),
        ("heap", "HeapOptions", size_of::<HeapOptions>()),
        ("heap", "SharedHeapOptions", size_of::<SharedHeapOptions>()),
        ("heap", "AllocationCache", size_of::<AllocationCache>()),
        ("heap", "GcWorker", size_of::<GcWorker>()),
        ("host", "ResourceTable", size_of::<ResourceTable>()),
        ("host", "BindingRegistry", size_of::<BindingRegistry>()),
        ("runtime", "EventLoop", size_of::<EventLoop>()),
        ("engine", "StaticSpace", size_of::<StaticSpace>()),
    ];

    eprintln!();
    eprintln!("runtime footprint: public component sizes");
    eprintln!("{:<12} {:<24} {:>12}", "module", "type", "size");
    eprintln!("{:-<12} {:-<24} {:-<12}", "", "", "");
    for (module, name, bytes) in rows {
        eprintln!("{module:<12} {name:<24} {:>12}", format_bytes(bytes as i64));
    }
}

/// Print retained allocation samples.
fn print_allocations(runtime: &RuntimeSetup, vm: VmSetup) {
    let mut world = runtime.world();
    let engine = runtime.engine();
    let runtime_spawn = ALLOCATOR.measure(|| runtime.spawn_runtime(&mut world, engine));
    let (mut world, runtime_id) = runtime.world_with_runtime();
    let engine = runtime.engine();
    let worker_spawn = ALLOCATOR.measure(|| runtime.spawn_worker(&mut world, runtime_id, engine));

    let machine_new = ALLOCATOR.measure(|| vm.machine());
    let mut machine = vm.machine();
    let continuation_yield = ALLOCATOR.measure(|| machine.yield_once());

    let mut machine = vm.machine();
    let continuation = machine.yield_once();
    let continuation_image = ALLOCATOR.measure(|| machine.continuation_image(&continuation));

    let mut machine = vm.machine();
    let continuation = machine.yield_once();
    let continuation_clone = ALLOCATOR.measure(|| {
        continuation
            .fork()
            .expect("footprint continuation should clone")
    });
    let rows = [
        ("world.new", ALLOCATOR.measure(|| runtime.world())),
        ("runtime.spawn.empty.vm", runtime_spawn),
        ("worker.spawn.empty.vm", worker_spawn),
        ("launch.empty", ALLOCATOR.measure(|| runtime.launch())),
        ("vm.machine.build", ALLOCATOR.measure(|| vm.build_machine())),
        ("vm.machine.new", machine_new),
        ("vm.continuation.yield", continuation_yield),
        ("vm.continuation.image", continuation_image),
        ("vm.continuation.clone", continuation_clone),
    ];

    eprintln!();
    eprintln!("runtime footprint: rust allocator samples");
    eprintln!(
        "{:<28} {:>12} {:>14} {:>14} {:>14}",
        "setup", "allocs", "allocated", "live", "peak"
    );
    eprintln!(
        "{:-<28} {:-<12} {:-<14} {:-<14} {:-<14}",
        "", "", "", "", ""
    );
    for (name, sample) in rows {
        print_allocation(name, sample);
    }
    eprintln!();
}

/// Print one step-by-step initialized VM machine allocation ledger.
fn print_vm_machine_breakdown(vm: VmSetup) {
    let (mut machine, machine_build) = ALLOCATOR.capture(|| vm.build_machine());
    let (mut statics, statics_empty) = ALLOCATOR.capture(StaticSpace::empty);
    let (heap, local_heap) = ALLOCATOR.capture(|| vm.local_heap());
    let (shared, shared_heap) = ALLOCATOR.capture(|| vm.shared_heap());
    let (_shared_gc, shared_gc) = ALLOCATOR.capture(|| shared.register_collector_worker());
    let (_shared_cache, shared_cache) = ALLOCATOR.capture(|| shared.allocation_cache());
    let initialize = ALLOCATOR.measure(|| {
        machine
            .initialize(&heap, &shared, &mut statics)
            .expect("footprint machine should initialize")
    });
    let rows = [
        ("vm.machine.build", machine_build),
        ("vm.static.empty", statics_empty),
        ("vm.local_heap.new", local_heap),
        ("vm.shared_heap.new", shared_heap),
        ("vm.shared_gc_worker.new", shared_gc),
        ("vm.shared_cache.new", shared_cache),
        ("vm.machine.initialize", initialize),
    ];

    eprintln!();
    eprintln!("runtime footprint: vm.machine.new allocation ledger");
    eprintln!(
        "{:<28} {:>12} {:>14} {:>14} {:>14}",
        "step", "allocs", "allocated B", "live B", "peak B"
    );
    eprintln!(
        "{:-<28} {:-<12} {:-<14} {:-<14} {:-<14}",
        "", "", "", "", ""
    );
    for (name, sample) in rows {
        print_allocation_bytes(name, sample);
    }
    print_allocation_bytes("total", sum_allocations(rows));
    eprintln!();
}

/// Print one allocation row.
fn print_allocation(name: &str, sample: AllocationSample) {
    eprintln!(
        "{name:<28} {:>12} {:>14} {:>14} {:>14}",
        sample.allocations,
        format_bytes(sample.allocated_bytes as i64),
        format_bytes(sample.live_bytes),
        format_bytes(sample.peak_live_bytes),
    );
}

/// Print one exact allocation row.
fn print_allocation_bytes(name: &str, sample: AllocationSample) {
    eprintln!(
        "{name:<28} {:>12} {:>14} {:>14} {:>14}",
        sample.allocations, sample.allocated_bytes, sample.live_bytes, sample.peak_live_bytes,
    );
}

/// Sum allocation rows.
fn sum_allocations(rows: [(&str, AllocationSample); 7]) -> AllocationSample {
    let mut total = AllocationSample {
        allocations: 0,
        allocated_bytes: 0,
        live_bytes: 0,
        peak_live_bytes: 0,
    };

    for (_, sample) in rows {
        total.allocations += sample.allocations;
        total.allocated_bytes += sample.allocated_bytes;
        total.live_bytes += sample.live_bytes;
        total.peak_live_bytes += sample.peak_live_bytes;
    }

    total
}

/// Format one byte count.
fn format_bytes(bytes: i64) -> String {
    let sign = if bytes < 0 { "-" } else { "" };
    let bytes = bytes.unsigned_abs() as f64;
    let units = ["B", "KiB", "MiB", "GiB"];
    let mut unit_index = 0usize;
    let mut value = bytes;

    while value >= 1024.0 && unit_index + 1 < units.len() {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{sign}{} {}", value as u64, units[unit_index])
    } else {
        format!("{sign}{value:.2} {}", units[unit_index])
    }
}
