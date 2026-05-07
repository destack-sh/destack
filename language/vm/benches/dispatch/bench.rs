mod arithmetic;
mod atomic;
mod benchmark;
mod fork;
mod memory;
mod program;
mod runtime;
mod tensor;
mod vector;

use criterion::{criterion_group, criterion_main};

use arithmetic::bench_integer_arithmetic;
use atomic::bench_atomic;
use fork::bench_fork;
use memory::bench_memory;
use tensor::bench_tensor;
use vector::bench_vector;

criterion_group!(
    benches,
    bench_integer_arithmetic,
    bench_vector,
    bench_tensor,
    bench_memory,
    bench_fork,
    bench_atomic
);
criterion_main!(benches);
