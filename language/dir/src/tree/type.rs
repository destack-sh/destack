use crate::{Expression, LocalNodeId, Parameter, WhereClause, WithClause};

/// The polymorphism of some type or declaration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Generics {
    /// The static parameters of the declaration.
    pub static_parameters: Option<Vec<LocalNodeId<Parameter>>> = None,
    /// The with clauses of the declaration.
    pub with_clauses: Option<Vec<LocalNodeId<WithClause>>> = None,
    /// The where clauses of the declaration.
    pub where_clauses: Option<Vec<LocalNodeId<WhereClause>>> = None,
}

impl Generics {
    /// Check whether the generics contain any clauses.
    pub fn is_empty(&self) -> bool {
        self.static_parameters.is_none()
            && self.with_clauses.is_none()
            && self.where_clauses.is_none()
    }
}

/// The polymoprhic relations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Heritage {
    /// The extends types of the declaration.
    pub extends_types: Option<Vec<LocalNodeId<Expression>>> = None,
    /// The implements types of the declaration.
    pub implements_types: Option<Vec<LocalNodeId<Expression>>> = None,
    /// The embedded types of the declaration.
    pub embedded_types: Option<Vec<LocalNodeId<Expression>>> = None,
}

impl Heritage {
    /// Check whether the heritage carries any relations.
    pub fn is_empty(&self) -> bool {
        self.extends_types.is_none()
            && self.implements_types.is_none()
            && self.embedded_types.is_none()
    }
}

/// A TypeBound is a type bound for a reference operation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VarianceBound {
    /// Implements a type (such that X implements Y, i.e. X implements Y).
    Implements,
    /// Extends a type (such that X is a subtype of Y, i.e. X <: Y).
    Extends,
    /// Super a type (such that X is a supertype of Y, i.e. X >: Y).
    Super,
}
