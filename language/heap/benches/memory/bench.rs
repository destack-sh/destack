mod access;
mod address;
mod allocation;
mod common;
mod field;
mod workload;

use criterion::{criterion_group, criterion_main};

use access::{bench_heap_access, bench_heap_scalar_access};
use address::bench_address_space;
use allocation::{
    bench_heap_allocation, bench_heap_allocation_matrix, bench_shared_parallel_allocation,
};
use field::bench_heap_field_access;
use workload::bench_heap_workload;

criterion_group!(
    benches,
    bench_address_space,
    bench_heap_allocation,
    bench_heap_allocation_matrix,
    bench_shared_parallel_allocation,
    bench_heap_access,
    bench_heap_scalar_access,
    bench_heap_field_access,
    bench_heap_workload
);
criterion_main!(benches);
