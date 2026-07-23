mod integer;
mod runtime;
mod tensor;
mod vector;

use criterion::{criterion_group, criterion_main};

use integer::bench_integer;
use tensor::bench_tensor;
use vector::bench_vector;

criterion_group!(benches, bench_integer, bench_vector, bench_tensor);
criterion_main!(benches);
