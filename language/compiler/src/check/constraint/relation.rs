use destack_dir as dir;

use crate::check::{
    CheckState, Condition, Constraint, Origin, TypeLiteralTerm, TypeOperand, TypeTerm, VariableId,
};

/// A relation between two type variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TypeRelation {
    /// Types must be equal.
    Equal,
    /// Source must be assignable to target.
    Assignable,
    /// Source must be explicitly castable to target.
    Castable,
    /// Value must satisfy a constraint.
    Satisfies,
    /// Subtype must extend supertype.
    Extends,
    /// Implementor must implement contract.
    Implements,
}

/// A relation between two static variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum StaticRelation {
    /// Static values must be equal.
    Equal,
    /// Source must be assignable to target.
    Assignable,
}

impl CheckState<'_> {
    /// Constrain two type variables.
    pub(in crate::check) fn add_type_constraint(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
        condition: Condition,
    ) {
        let constraint = Constraint::Type {
            relation,
            left: left.into(),
            right: right.into(),
            origin,
            condition,
        };

        self.add_constraint(constraint);
    }

    /// Constrain one expression condition to boolean.
    pub(in crate::check) fn constrain_condition(
        &mut self,
        source: dir::LocalNodeIdAny,
        condition: VariableId,
        static_condition: Condition,
    ) {
        let origin = Origin::Node(source.into_global(condition.module));
        let expected = self
            .terms
            .push(TypeTerm::Literal(TypeLiteralTerm::boolean()));

        self.add_type_constraint(
            origin,
            TypeRelation::Assignable,
            condition,
            expected,
            static_condition,
        );
    }
}
