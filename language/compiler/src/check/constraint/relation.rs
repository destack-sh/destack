use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Condition, Constraint, Origin, StaticTerm, TypeLiteralTerm, TypeOperand, TypeTerm,
    VariableId,
};

/// Relation between two type operands.
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

/// Relation between two static operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum StaticRelation {
    /// Static values must be equal.
    Equal,
    /// Source must be assignable to target.
    Assignable,
}

impl CheckState<'_> {
    /// Equate one type variable to one type term.
    pub(in crate::check) fn equate_type(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
        condition: Condition,
    ) {
        let origin = self.variable(variable).source;
        let term = self.inference.terms.push(term);

        self.relate_type(origin, TypeRelation::Equal, variable, term, condition);
    }

    /// Equate one static variable to one static term.
    pub(in crate::check) fn equate_static(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
        condition: Condition,
    ) {
        let origin = self.variable(variable).source;
        let term = self.inference.terms.push(term);
        let constraint = Constraint::Static {
            relation: StaticRelation::Equal,
            left: variable.into(),
            right: term.into(),
            origin,
            condition,
        };

        self.push_constraint(constraint);
    }

    /// Relate two type operands.
    pub(in crate::check) fn relate_type(
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

        self.push_constraint(constraint);
    }

    /// Expect one expression condition to be boolean.
    pub(in crate::check) fn constrain_condition(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        condition: TypeOperand,
        static_condition: Condition,
    ) {
        let origin = Origin::Node(source.into_global(module));
        let expected = self
            .inference
            .terms
            .push(TypeTerm::Literal(TypeLiteralTerm::boolean()));

        self.relate_type(
            origin,
            TypeRelation::Assignable,
            condition,
            expected,
            static_condition,
        );
    }
}
