//! Cranelift codegen tests.

mod arithmetic;
mod block;
mod function;
mod memory;

mod tests;

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
