use crate::{Asynchrony, Keyword, NodeId, Parameter, Type};

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
    /// New constructor method.
    New,
    /// Implicit call method.
    Call,
}

impl FunctionMode {
    /// Get the keyword for the function accessor.
    #[inline]
    pub fn to_keyword(&self) -> Option<Keyword> {
        match self {
            FunctionMode::Getter => Some(Keyword::Get),
            FunctionMode::Setter => Some(Keyword::Set),
            FunctionMode::Constructor => Some(Keyword::Constructor),
            FunctionMode::New => Some(Keyword::New),
            FunctionMode::Call => None,
        }
    }
}

/// The abstraction level of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionAbstraction {
    /// Abstract function.
    Abstract,
    /// Abstract override.
    AbstractOverride,
    /// Concrete override.
    ConcreteOverride,
    /// Concrete function.
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
    pub static_parameters: Option<Vec<NodeId<Parameter>>>,
    /// The dynamic parameters of the function.
    pub dynamic_parameters: Vec<NodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<NodeId<Type>>,
}
