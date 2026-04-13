use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, FunctionMode, GenericParameter, LocalNodeId, Parameter, TypeExpression, WhereClause,
};

/// The cardinality of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionCardinality {
    /// Scalar function.
    Scalar,
    /// Generator function.
    Generator,
}

/// The style of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionKind {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}

/// The signature of a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// The where clauses of the function.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The optional `this` parameter.
    pub this_parameter: Option<LocalNodeId<Parameter>>,
    /// The runtime parameters of the function.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the function.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
}
