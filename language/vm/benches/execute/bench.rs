mod integer;
mod runtime;
mod vector;

use criterion::{criterion_group, criterion_main};

use integer::bench_integer;
use vector::bench_vector;

criterion_group!(benches, bench_integer, bench_vector);
criterion_main!(benches);
