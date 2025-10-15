use dyst_dir::{Destination, Expression, NodeId, NodeIdAny, Path, Type};

use crate::{AstNodeIdAny, Compiler, CompilerMode};

/// Request to statically evaluate something to a value.
#[derive(Debug, Clone)]
pub enum EvaluateRequest {
    /// Evaluate a Type to its Type value.
    EvaluateType { ty: NodeId<Type> },
    /// Evaluate an Expression to its result value.
    EvaluateExpression { expression: NodeId<Expression> },
    /// Evaluate a Destination to its result value.
    EvaluateDestination {
        scope_id: NodeIdAny,
        destination: Destination,
    },
    /// Evaluate a Path to its result value.
    EvaluatePath { scope_id: NodeIdAny, path: Path },
}

/// Request to check something statically.
#[derive(Debug, Clone)]
pub enum CheckRequest {
    /// Typecheck a node.
    CheckType { node: NodeIdAny },
}

/// Request to lower something.
#[derive(Debug, Clone)]
pub enum LowerRequest {
    /// Lower a node to DIR.
    LowerToDir { node: AstNodeIdAny },
    /// Lower a node to MIR.
    LowerToMir { node: AstNodeIdAny },
}

/// Message from the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerMessage {
    /// Request to evaluate something statically.
    EvaluateRequest(EvaluateRequest),
    /// Request to check something statically.
    CheckRequest(CheckRequest),
    /// Request to lower something.
    LowerRequest(LowerRequest),
}

impl<'s> Compiler<'s> {
    /// Evaluate compile time constructs and check compile time invariants.
    /// Runs until there is nothing left to evaluate.
    pub fn compile(&mut self) {
        assert!(self.mode == CompilerMode::Parsed);
        self.mode = CompilerMode::Compiling;

        // todo!("Compiler.evaluate({self:?})");

        self.mode = CompilerMode::Compiled;
    }
}
