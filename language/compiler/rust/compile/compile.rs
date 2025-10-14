use dyst_dir::{Expression, NodeId, NodeIdAny, Type};

use crate::{Compiler, CompilerMode};

/// Request to statically evaluate something to a value.
#[derive(Debug, Clone)]
pub enum EvaluateRequest {
    /// Evaluate a Type to its Type value.
    EvaluateType { ty: NodeId<Type> },
    /// Evaluate an Expression to its result value.
    EvaluateExpression { expression: NodeId<Expression> },
}

/// Request to check something statically.
#[derive(Debug, Clone)]
pub enum CheckRequest {
    /// Typecheck a node.
    CheckType { node: NodeIdAny },
}

/// Request to lower something to MIR.
#[derive(Debug, Clone)]
pub enum LowerRequest {}

/// Message from the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerMessage {
    /// Request to evaluate something statically.
    EvaluateRequest(EvaluateRequest),
    /// Request to check something statically.
    CheckRequest(CheckRequest),
    /// Request to lower something to MIR.
    LowerRequest(LowerRequest),
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
