use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, FunctionRole, GenericParameter, LocalNodeId, Parameter, TypeExpression, WhereClause,
};

/// The source form of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionForm {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}

/// The signature of a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// The asynchrony of the function.
    pub asynchrony: Asynchrony,
    /// The special role of the function.
    pub role: Option<FunctionRole>,
    /// The source form of the function.
    pub form: FunctionForm,
    /// The generic parameters of the function.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the function.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The optional `this` parameter.
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
