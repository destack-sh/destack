use crate::{Asynchrony, FunctionMode, Generics, LocalNodeId, Parameter, Type};

/// The cardinality of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionCardinality {
    /// Scalar function.
    Scalar,
    /// Generator function.
    Generator,
}

/// The abstraction level of a declaration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionAbstraction {
    /// Abstract declaration.
    Abstract,
    /// Abstract override.
    AbstractOverride,
    /// Concrete override.
    ConcreteOverride,
    /// Concrete declaration.
    Concrete,
}

/// The style of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionKind {
    /// Function with a body.
    Function,
    /// Lambda function with a return type.
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
    pub dynamic_parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<LocalNodeId<Type>>,
}
