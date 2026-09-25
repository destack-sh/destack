use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    Asynchrony, ConstructorType, FunctionRole, FunctionTypeExpression, GenericParameter,
    LocalNodeId, NodeFold, Parameter, TypeExpression, WhereClause,
};

/// The source form of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum FunctionForm {
    /// A normal function.
    Function,
    /// A lambda function.
    Lambda,
}

/// When a function may be called.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum FunctionPhase {
    /// The function may be called at runtime and during const evaluation.
    Normal,
    /// The function may only be called during const evaluation.
    Const,
}

/// The source form used to spell a receiver.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ThisForm {
    /// Shorthand receiver, for example `&readonly this`.
    Implicit,
    /// `this: T`.
    Explicit,
}

/// The signature of a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, NodeFold)]
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
    /// The value parameter source nodes of the function.
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
    /// Convert this call signature into a function type.
    pub fn into_function_type(self) -> FunctionTypeExpression {
        debug_assert!(self.role.is_none() || self.role == Some(FunctionRole::Call));

        FunctionTypeExpression {
            generic_parameters: self.generic_parameters,
            where_clauses: self.where_clauses,
            this_form: self.this_form,
            this_parameter: self.this_parameter,
            parameters: self.parameters,
            return_type: self.return_type,
        }
    }

    /// Convert this constructor signature into a constructor type.
    pub fn into_constructor_type(self) -> ConstructorType {
        debug_assert!(matches!(
            self.role,
            Some(FunctionRole::Constructor | FunctionRole::New)
        ));

        ConstructorType {
            generic_parameters: self.generic_parameters,
            where_clauses: self.where_clauses,
            parameters: self.parameters,
            return_type: self.return_type,
            is_abstract: self.is_abstract,
        }
    }

    /// Return whether this signature declares a generic scope.
    pub fn declares_generic_scope(&self) -> bool {
        !self.generic_parameters.is_empty() || !self.where_clauses.is_empty()
    }

    /// Return whether this signature constructs its receiver.
    pub fn is_constructor(&self) -> bool {
        matches!(
            self.role,
            Some(FunctionRole::Constructor | FunctionRole::New)
        )
    }
}
