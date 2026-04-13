use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{LocalNodeId, Parameter, TypeExpression, WhereClause};

/// The polymorphism of some type or declaration.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, AdaptImage)]
pub struct Generics {
    /// The static parameters of the declaration.
    pub static_parameters: Option<Vec<LocalNodeId<Parameter>>>,
    /// The where clauses of the declaration.
    pub where_clauses: Option<Vec<LocalNodeId<WhereClause>>>,
}

impl Generics {
    /// Check whether the generics contain any clauses.
    pub fn is_empty(&self) -> bool {
        self.static_parameters.is_none() && self.where_clauses.is_none()
    }
}

/// The syntactic heritage of a type declaration (what the user wrote).
///
/// Heritage stores type-expression node IDs that represent the extends/implements/embedded clauses.
/// These expressions get resolved during the resolve phase like any other type expressions.
/// During analyze phase, the resolved symbols are extracted into `Lineage` (in TypeTable).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, AdaptImage)]
pub struct Heritage {
    /// The extends types of the declaration.
    pub extends_types: Option<Vec<LocalNodeId<TypeExpression>>>,
    /// The implements types of the declaration.
    pub implements_types: Option<Vec<LocalNodeId<TypeExpression>>>,
    /// The embedded types of the declaration.
    pub embedded_types: Option<Vec<LocalNodeId<TypeExpression>>>,
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
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum VarianceBound {
    /// Implements a type (such that X implements Y, i.e. X implements Y).
    Implements,
    /// Extends a type (such that X is a subtype of Y, i.e. X <: Y).
    Extends,
    /// Super a type (such that X is a supertype of Y, i.e. X >: Y).
    Super,
}

/// The subtyping relation.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum SubtypingMode {
    /// Contravariant (such that X is a subtype of Y, i.e. X <: Y).
    Contravariant,
    /// Covariant (such that X is a supertype of Y, i.e. X >: Y).
    Covariant,
    /// Invariant (such that X is not a subtype of Y, i.e. X !<: Y).
    Invariant,
}
