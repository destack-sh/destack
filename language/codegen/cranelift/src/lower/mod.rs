//! MIR to Cranelift IR lowering.
//!
//! This module handles the translation from Destack's MIR to Cranelift IR.
//! The lowering is organized in two layers:
//!
//! - `ModuleLowerer`: handles module-level concerns like function declarations
//! - `FunctionLowerer`: handles the translation of individual function bodies

mod function;
mod module;
pub(crate) mod r#type;

pub(crate) use function::*;
pub(crate) use module::*;
