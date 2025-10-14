use dyst_dir::{Expression, NodeId, Type};

use crate::{Compiler, CompilerMode};

/// Request to statically evaluate something to a value.
#[derive(Debug, Clone)]
pub enum EvaluateRequest {
    /// Evaluate a Type.
    EvaluateType { ty: NodeId<Type> },
    /// Evaluate an Expression.
    EvaluateExpression { expression: NodeId<Expression> },
}

/// Request to check something statically.
#[derive(Debug, Clone)]
pub enum CheckRequest {
    /// Check a Type.
    CheckType,
}

/// Message from the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerMessage {
    /// Request to evaluate something statically.
    EvaluateRequest(EvaluateRequest),
    /// Request to check something statically.
    CheckRequest(CheckRequest),
}

impl<'s> Compiler<'s> {
    /// Evaluate compile time constructs and check all compile time invariants.
    /// Runs until there is nothing left to evaluate.
    pub fn compile(&mut self) {
        assert!(self.mode == CompilerMode::Parsed);
        self.mode = CompilerMode::Compiling;

        // todo!("Compiler.evaluate({self:?})");

        self.mode = CompilerMode::Compiled;
    }
}
