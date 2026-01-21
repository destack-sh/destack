//! Cranelift codegen tests.

mod aggregate;
mod allocate;
mod arithmetic;
mod block;
mod call;
mod constant;
mod function;
mod memory;

mod tests;

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
