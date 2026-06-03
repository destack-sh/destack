use crate::{BinaryOperator, Block, Function, LocalNodeId, Type, TypeReference, Value};

use super::Variable;

/// Result type for MIR builder operations.
pub type BuildResult<T> = Result<T, BuildError>;

/// Error produced when MIR builder preconditions are not satisfied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// A declared function already has a body.
    FunctionAlreadyHasBody {
        /// The function being rebuilt.
        function: LocalNodeId<Function>,
    },
    /// A builder operation requires a current block.
    MissingCurrentBlock,
    /// A function was finished before any block was created.
    MissingEntryBlock {
        /// The function being finished.
        function: LocalNodeId<Function>,
    },
    /// A value has no type in the function value table.
    MissingValueType {
        /// The value whose type was requested.
        value: Value,
        /// The builder operation requesting the type.
        context: String,
    },
    /// A type reference is missing where a concrete type is required.
    MissingTypeReference {
        /// The builder operation requesting the type.
        context: String,
    },
    /// A malformed type reference appeared where a concrete type is required.
    ErrorTypeReference {
        /// The builder operation requesting the type.
        context: String,
    },
    /// A field index does not exist on an aggregate type.
    InvalidFieldIndex {
        /// The aggregate type being indexed.
        aggregate: LocalNodeId<Type>,
        /// The requested field index.
        index: u32,
    },
    /// A field operation was applied to a non-field aggregate type.
    InvalidFieldOwner {
        /// The type being accessed.
        ty: LocalNodeId<Type>,
    },
    /// An element operation was applied to a non-indexed aggregate type.
    InvalidElementOwner {
        /// The type being accessed.
        ty: LocalNodeId<Type>,
    },
    /// A vector operation was applied to a non-vector type.
    InvalidVectorOwner {
        /// The type being accessed.
        ty: LocalNodeId<Type>,
    },
    /// A tensor operation was applied to a non-tensor type.
    InvalidTensorOwner {
        /// The type being accessed.
        ty: LocalNodeId<Type>,
    },
    /// A tensor view operation was applied to a non-view type.
    InvalidTensorViewOwner {
        /// The type being accessed.
        ty: LocalNodeId<Type>,
    },
    /// A count is too large for MIR instruction metadata.
    CountTooLarge {
        /// The count that could not fit.
        count: usize,
        /// The builder operation storing the count.
        context: String,
    },
    /// A callable type does not expose a function signature.
    MissingFunctionSignature {
        /// The callable type.
        ty: LocalNodeId<Type>,
    },
    /// A function entry block has parameters that differ from the function parameters.
    EntryParameterMismatch,
    /// A function parameter was not represented by a concrete SSA value.
    MissingConcreteFunctionParameter {
        /// The requested parameter index.
        index: usize,
    },
    /// SSA construction needs a variable type that was never declared.
    MissingVariableType {
        /// The variable whose type was requested.
        variable: Variable,
    },
    /// A binary operation received mismatched operand types.
    MismatchedBinaryOperands {
        /// The binary operator being inserted.
        operator: BinaryOperator,
        /// The left operand type.
        left: LocalNodeId<Type>,
        /// The right operand type.
        right: LocalNodeId<Type>,
    },
    /// A select operation received mismatched value types.
    MismatchedSelectOperands {
        /// The then value type.
        then_type: LocalNodeId<Type>,
        /// The else value type.
        else_type: LocalNodeId<Type>,
    },
    /// A closure environment was requested with a different type than the existing environment.
    MismatchedClosureEnvironment {
        /// The environment type already recorded on the function.
        existing: TypeReference,
        /// The newly requested environment type.
        requested: LocalNodeId<Type>,
    },
    /// SSA construction read an undefined variable in a sealed block with no predecessors.
    UndefinedVariable {
        /// The variable being read.
        variable: Variable,
        /// The block where the variable was read.
        block: LocalNodeId<Block>,
    },
    /// SSA construction could not find a predecessor edge for a required phi argument.
    MissingPhiPredecessorEdge {
        /// The predecessor block.
        from: LocalNodeId<Block>,
        /// The successor block.
        to: LocalNodeId<Block>,
    },
}

impl std::fmt::Display for BuildError {
    /// Format this builder error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FunctionAlreadyHasBody { function } => {
                write!(formatter, "function already has a body: {function:?}")
            }
            Self::MissingCurrentBlock => {
                write!(formatter, "builder operation requires a current block")
            }
            Self::MissingEntryBlock { function } => {
                write!(formatter, "function has no entry block: {function:?}")
            }
            Self::MissingValueType { value, context } => {
                write!(formatter, "missing value type for {context}: {value:?}")
            }
            Self::MissingTypeReference { context } => {
                write!(formatter, "missing type reference for {context}")
            }
            Self::ErrorTypeReference { context } => {
                write!(formatter, "malformed type reference for {context}")
            }
            Self::InvalidFieldIndex { aggregate, index } => {
                write!(
                    formatter,
                    "field index {index} is out of bounds for aggregate type {aggregate:?}"
                )
            }
            Self::InvalidFieldOwner { ty } => {
                write!(
                    formatter,
                    "field access expects an aggregate type, got {ty:?}"
                )
            }
            Self::InvalidElementOwner { ty } => {
                write!(
                    formatter,
                    "element access expects an indexed collection type, got {ty:?}"
                )
            }
            Self::InvalidVectorOwner { ty } => {
                write!(formatter, "vector access expects a vector type, got {ty:?}")
            }
            Self::InvalidTensorOwner { ty } => {
                write!(formatter, "tensor access expects a tensor type, got {ty:?}")
            }
            Self::InvalidTensorViewOwner { ty } => {
                write!(
                    formatter,
                    "tensor access expects a tensor view type, got {ty:?}"
                )
            }
            Self::CountTooLarge { count, context } => {
                write!(
                    formatter,
                    "{context} is too large for MIR metadata: {count}"
                )
            }
            Self::MissingFunctionSignature { ty } => {
                write!(formatter, "callable type has no function signature: {ty:?}")
            }
            Self::EntryParameterMismatch => {
                write!(
                    formatter,
                    "entry block parameters must match function parameters"
                )
            }
            Self::MissingConcreteFunctionParameter { index } => {
                write!(
                    formatter,
                    "function parameter {index} has no concrete SSA value"
                )
            }
            Self::MissingVariableType { variable } => {
                write!(formatter, "missing type for SSA variable {variable}")
            }
            Self::MismatchedBinaryOperands {
                operator,
                left,
                right,
            } => {
                write!(
                    formatter,
                    "binary operator {operator:?} expects matching operand types: {left:?} and {right:?}"
                )
            }
            Self::MismatchedSelectOperands {
                then_type,
                else_type,
            } => {
                write!(
                    formatter,
                    "select expects matching value types: {then_type:?} and {else_type:?}"
                )
            }
            Self::MismatchedClosureEnvironment {
                existing,
                requested,
            } => {
                write!(
                    formatter,
                    "closure environment type mismatch: existing {existing:?}, requested {requested:?}"
                )
            }
            Self::UndefinedVariable { variable, block } => {
                write!(
                    formatter,
                    "undefined SSA variable {variable} read in block {block:?}"
                )
            }
            Self::MissingPhiPredecessorEdge { from, to } => {
                write!(
                    formatter,
                    "missing phi predecessor edge from block {from:?} to block {to:?}"
                )
            }
        }
    }
}

impl std::error::Error for BuildError {}
