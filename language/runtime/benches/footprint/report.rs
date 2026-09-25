use std::mem::size_of;
use std::sync::Once;

use tspp_heap::{
    AllocationCache, Heap, HeapOptions, SharedHeap, SharedHeapOptions, SharedMarkWorker,
    SizeClassTable,
};
use tspp_memory::MemoryMap;
use tspp_program::StaticSpace;
use tspp_repository::{Environment, RuntimeOptions};
use tspp_runtime::binding::BindingTable;
use tspp_runtime::diagnostic::DiagnosticStore;
use tspp_runtime::host::resource::ResourceTable;
use tspp_runtime::runtime::Runtime;
use tspp_runtime::worker::{RunnableScope, Worker};
use tspp_runtime::world::observation::{
    Observation, ObservationEntry, ObservationLog, ObservationScope,
};
use tspp_runtime::world::random::Random;
use tspp_runtime::world::topology::LabelSet;
use tspp_runtime::world::trace::{
    ClockTrace, EntropySubject, RandomTrace, Trace, TraceEntry, TraceLog,
};
use tspp_runtime::world::{Entity, Policy, World};
use tspp_vm::Machine;

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
    });
}

/// Print static type sizes.
fn print_type_sizes() {
    let rows = [
        ("world", "World", size_of::<World>()),
        ("world", "Policy", size_of::<Policy>()),
        ("world", "Trace", size_of::<Trace>()),
        ("world", "TraceEntry", size_of::<TraceEntry>()),
        ("world", "ClockTrace", size_of::<ClockTrace>()),
        ("world", "RandomTrace", size_of::<RandomTrace>()),
        ("world", "EntropySubject", size_of::<EntropySubject>()),
        ("world", "TraceLog", size_of::<TraceLog>()),
        ("world", "Observation", size_of::<Observation>()),
        ("world", "ObservationEntry", size_of::<ObservationEntry>()),
        ("world", "LabelSet", size_of::<LabelSet>()),
        ("world", "ObservationScope", size_of::<ObservationScope>()),
        ("world", "ObservationLog", size_of::<ObservationLog>()),
        ("world", "Entity", size_of::<Entity>()),
        ("runtime", "Runtime", size_of::<Runtime>()),
        ("runtime", "Worker", size_of::<Worker>()),
        ("runtime", "RunnableScope", size_of::<RunnableScope>()),
        ("runtime", "Random", size_of::<Random>()),
        ("host", "ResourceTable", size_of::<ResourceTable>()),
        ("host", "BindingTable", size_of::<BindingTable>()),
        (
            "diagnostic",
            "DiagnosticStore",
            size_of::<DiagnosticStore>(),
        ),
        ("workspace", "Environment", size_of::<Environment>()),
        ("workspace", "RuntimeOptions", size_of::<RuntimeOptions>()),
        ("machine", "StaticSpace", size_of::<StaticSpace>()),
        ("vm", "Machine", size_of::<Machine>()),
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
        ("memory", "MemoryMap", size_of::<MemoryMap>()),
        ("heap", "SizeClassTable", size_of::<SizeClassTable>()),
        ("heap", "HeapOptions", size_of::<HeapOptions>()),
        ("heap", "SharedHeapOptions", size_of::<SharedHeapOptions>()),
        ("heap", "AllocationCache", size_of::<AllocationCache>()),
        ("heap", "SharedMarkWorker", size_of::<SharedMarkWorker>()),
        ("host", "ResourceTable", size_of::<ResourceTable>()),
        ("host", "BindingTable", size_of::<BindingTable>()),
        ("machine", "StaticSpace", size_of::<StaticSpace>()),
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
    let worker_spawn = ALLOCATOR.measure(|| runtime.spawn_worker(&mut world, runtime_id));

    let machine_new = ALLOCATOR.measure(|| vm.machine());
    let mut machine = vm.machine();
    let machine_run = ALLOCATOR.measure(|| machine.run());
    let rows = [
        ("world.new", ALLOCATOR.measure(|| runtime.world())),
        ("runtime.spawn.empty.vm", runtime_spawn),
        ("worker.spawn.empty.vm", worker_spawn),
        ("execution.empty", ALLOCATOR.measure(|| runtime.execute())),
        ("vm.machine.build", ALLOCATOR.measure(|| vm.build_machine())),
        ("vm.machine.new", machine_new),
        ("vm.machine.run", machine_run),
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
