use crate::{Asynchrony, Expression, FunctionMode, Generics, NodeId, Parameter};

/// The cardinality of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionCardinality {
    /// Scalar function.
    Scalar,
    /// Generator function.
    Generator,
}

/// The abstraction level of a definition.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionAbstraction {
    /// Abstract definition.
    Abstract,
    /// Abstract override.
    AbstractOverride,
    /// Concrete override.
    ConcreteOverride,
    /// Concrete definition.
    Concrete,
}

/// A FunctionKind is the style of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionKind {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}

/// The signature of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// The abstraction level of the function.
    pub abstraction: FunctionAbstraction,
    /// The asynchrony of the function.
    pub asynchrony: Asynchrony,
    /// The cardinality of the function.
    pub cardinality: FunctionCardinality,
    /// The mode of the function.
    pub mode: Option<FunctionMode>,
    /// The kind of the function.
    pub kind: FunctionKind,
    /// The generics of the function.
    pub generics: Option<Generics>,
    /// The dynamic parameters of the function.
    pub dynamic_parameters: Vec<NodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<Expression>>,
}
