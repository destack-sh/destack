mod allocation;
mod config;
mod fork;
mod graph;
mod heap;
mod map;
mod trace;
mod workload;

use criterion::{criterion_group, criterion_main};

use allocation::{
    bench_heap_allocation, bench_heap_allocation_matrix, bench_shared_parallel_allocation,
};
use map::bench_memory_map;
use workload::bench_heap_workload;

criterion_group!(
    benches,
    bench_memory_map,
    bench_heap_allocation,
    bench_heap_allocation_matrix,
    bench_shared_parallel_allocation,
    bench_heap_workload
);
criterion_main!(benches);
