use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, FunctionRole, GenericParameter, LocalNodeId, Parameter, TypeExpression, View,
    WhereClause,
};

/// The source form of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionForm {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}

/// When a function may be called.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionPhase {
    /// The function may be called at runtime and evaluated at comptime.
    Normal,
    /// The function may only be called during comptime evaluation.
    Comptime,
}

/// The source form used to spell a receiver.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThisForm {
    /// Shorthand receiver, for example `&readonly this`.
    Implicit,
    /// `this: T`.
    Explicit,
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
    /// When the function may be called.
    pub phase: FunctionPhase,
    /// The generic parameters of the function.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the function.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The source form used for the receiver.
    pub this_form: Option<ThisForm>,
    /// The optional `this` parameter.
    pub this_parameter: Option<LocalNodeId<Parameter>>,
    /// The dynamic parameters of the function.
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

impl FunctionSignature {
    /// Return whether this signature declares a generic template.
    pub fn declares_generic_template(&self, view: &View<'_>) -> bool {
        // explicit generic header
        if !self.generic_parameters.is_empty() {
            return true;
        }

        // comptime receiver parameter
        if let Some(parameter) = self.this_parameter
            && view.get(parameter).is_comptime()
        {
            return true;
        }

        // comptime parameters
        self.parameters
            .iter()
            .any(|parameter| view.get(*parameter).is_comptime())
    }
}
