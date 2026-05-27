use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    AssignPatternTerm, CheckState, Constraint, ConstraintOrigin, PatternTerm, VariableId,
};

/// Relation between a value type and a pattern.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternRelation {
    /// Pattern matches a value.
    Match(PatternTerm),
    /// Assignment pattern accepts an assigned value.
    Assign(AssignPatternTerm),
}

impl PatternRelation {
    /// Return variables referenced by this relation.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Match(pattern) => pattern.referenced_variables(),
            Self::Assign(pattern) => pattern.referenced_variables(),
        }
    }
}

impl CheckState<'_> {
    /// Constrain one pattern against one value type.
    pub(in crate::check) fn constrain_pattern(
        &mut self,
        relation: PatternRelation,
        source: dir::LocalNodeIdAny,
        value: VariableId,
    ) {
        let origin = ConstraintOrigin::Node(source.into_global(self.input.module_id));
        let constraint = Constraint::RelatePattern {
            relation,
            value,
            origin,
            condition: self.active_static_condition(),
        };

        self.add_constraint(constraint);
    }
}
