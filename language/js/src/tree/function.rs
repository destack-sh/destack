use crate::{Asynchrony, GenericParameter, Keyword, LocalNodeId, Parameter, TypeExpression};

/// The cardinality of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionCardinality {
    /// Scalar function.
    Scalar,
    /// Generator function.
    Generator,
}

/// The mode of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionMode {
    /// Getter method.
    Getter,
    /// Setter method.
    Setter,
    /// Constructor method.
    Constructor,
}

impl FunctionMode {
    /// Get the keyword for the function accessor.
    #[inline]
    pub fn to_keyword(&self) -> Option<Keyword> {
        match self {
            FunctionMode::Getter => Some(Keyword::Get),
            FunctionMode::Setter => Some(Keyword::Set),
            FunctionMode::Constructor => Some(Keyword::Constructor),
        }
    }
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
    /// Whether the function is abstract.
    pub is_abstract: bool,
    /// Whether the function is an override.
    pub is_override: bool,
    /// The asynchrony of the function.
    pub asynchrony: Asynchrony,
    /// The cardinality of the function.
    pub cardinality: FunctionCardinality,
    /// The mode of the function.
    pub mode: Option<FunctionMode>,
    /// The kind of the function.
    pub kind: FunctionKind,
    /// The generic parameters of the function.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The optional `this` parameter of the function.
    pub this_parameter: Option<LocalNodeId<Parameter>>,
    /// The runtime parameters of the function.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
}
