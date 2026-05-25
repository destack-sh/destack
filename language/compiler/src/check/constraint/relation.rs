use crate::check::{CheckModuleState, Constraint, ConstraintOrigin, VariableId};

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
}

impl CheckModuleState {
    /// Relate two type variables.
    pub(in crate::check) fn relate_type(
        &mut self,
        origin: ConstraintOrigin,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
    ) {
        let constraint = Constraint::RelateType {
            relation,
            left,
            right,
            origin,
        };

        self.add_constraint(constraint);
    }

    /// Relate two static variables.
    pub(in crate::check) fn relate_static(
        &mut self,
        origin: ConstraintOrigin,
        relation: StaticRelation,
        left: VariableId,
        right: VariableId,
    ) {
        let constraint = Constraint::RelateStatic {
            relation,
            left,
            right,
            origin,
        };

        self.add_constraint(constraint);
    }
}
