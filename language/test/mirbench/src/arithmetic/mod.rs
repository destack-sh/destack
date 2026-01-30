use super::Program;

mod binary_float;
mod binary_int_mix;
mod cast_mix;
mod collatz;
mod fib_iterative;
mod fib_recursive;
mod mandelbrot_sum;
mod matmul_32;
mod matmul_64;
mod nbody;
mod poly_eval;
mod prime_sieve;
mod rolling_hash;
mod rolling_stats;
mod saxpy;
mod unary_ops;
mod xorshift_sum;

pub use binary_float::BINARY_FLOAT;
pub use binary_int_mix::BINARY_INT_MIX;
pub use cast_mix::CAST_MIX;
pub use collatz::COLLATZ;
pub use fib_iterative::FIB_ITERATIVE;
pub use fib_recursive::FIB_RECURSIVE;
pub use mandelbrot_sum::MANDELBROT_SUM;
pub use matmul_32::MATMUL_32;
pub use matmul_64::MATMUL_64;
pub use nbody::NBODY;
pub use poly_eval::POLY_EVAL;
pub use prime_sieve::PRIME_SIEVE;
pub use rolling_hash::ROLLING_HASH;
pub use rolling_stats::ROLLING_STATS;
pub use saxpy::SAXPY;
pub use unary_ops::UNARY_OPS;
pub use xorshift_sum::XORSHIFT_SUM;

/// All benchmark programs in this category.
pub const ALL: &[&Program] = &[
    &FIB_RECURSIVE,
    &FIB_ITERATIVE,
    &PRIME_SIEVE,
    &POLY_EVAL,
    &ROLLING_HASH,
    &ROLLING_STATS,
    &SAXPY,
    &BINARY_INT_MIX,
    &XORSHIFT_SUM,
    &BINARY_FLOAT,
    &COLLATZ,
    &UNARY_OPS,
    &CAST_MIX,
    &MATMUL_32,
    &MATMUL_64,
    &NBODY,
    &MANDELBROT_SUM,
];
