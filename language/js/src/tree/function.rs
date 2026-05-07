use crate::{Asynchrony, GenericParameter, Keyword, LocalNodeId, Parameter, TypeExpression};

use serde::{Deserialize, Serialize};

/// The role of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionRole {
    /// Getter method.
    Getter,
    /// Setter method.
    Setter,
    /// Constructor method.
    Constructor,
}

impl FunctionRole {
    /// Get the keyword for the function accessor.
    #[inline]
    pub fn to_keyword(&self) -> Option<Keyword> {
        match self {
            FunctionRole::Getter => Some(Keyword::Get),
            FunctionRole::Setter => Some(Keyword::Set),
            FunctionRole::Constructor => Some(Keyword::Constructor),
        }
    }
}

/// The style of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionForm {
    /// Function with a body.
    Function,
    /// Lambda function with a return type.
    Lambda,
}

/// The signature of a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// The asynchrony of the function.
    pub asynchrony: Asynchrony,
    /// The role of the function.
    pub role: Option<FunctionRole>,
    /// The form of the function.
    pub form: FunctionForm,
    /// The generic parameters of the function.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The optional `this` parameter of the function.
    pub this_parameter: Option<LocalNodeId<Parameter>>,
    /// The runtime parameters of the function.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
    /// Whether the function is abstract.
    pub is_abstract: bool,
    /// Whether the function is an override.
    pub is_override: bool,
    /// Whether the function is a generator.
    pub is_generator: bool,
}
