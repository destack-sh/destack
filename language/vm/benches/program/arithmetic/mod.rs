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
mod prime_sieve;
mod unary_ops;
mod xorshift_sum;

pub(crate) use binary_float::BINARY_FLOAT;
pub(crate) use binary_int_mix::BINARY_INT_MIX;
pub(crate) use cast_mix::CAST_MIX;
pub(crate) use collatz::COLLATZ;
pub(crate) use fib_iterative::FIB_ITERATIVE;
pub(crate) use fib_recursive::FIB_RECURSIVE;
pub(crate) use mandelbrot_sum::MANDELBROT_SUM;
pub(crate) use matmul_32::MATMUL_32;
pub(crate) use matmul_64::MATMUL_64;
pub(crate) use nbody::NBODY;
pub(crate) use prime_sieve::PRIME_SIEVE;
pub(crate) use unary_ops::UNARY_OPS;
pub(crate) use xorshift_sum::XORSHIFT_SUM;

/// All benchmark programs in this category.
pub(crate) const ALL: &[&Program] = &[
    &FIB_RECURSIVE,
    &FIB_ITERATIVE,
    &PRIME_SIEVE,
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
