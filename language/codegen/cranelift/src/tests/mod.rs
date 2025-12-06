//! Cranelift codegen tests.

mod arithmetic;
mod block;
mod function;

mod tests;

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
