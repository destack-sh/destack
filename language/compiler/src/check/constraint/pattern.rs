use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    AssignPatternTerm, CheckState, Condition, Constraint, Origin, PatternTerm, TermId, TypeOperand,
    VariableId,
};

/// Relation between a value type and a pattern.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternRelation {
    /// Match pattern checks a value.
    Match(TermId<PatternTerm>),
    /// Assignment pattern accepts an assigned value.
    Assign(TermId<AssignPatternTerm>),
}

impl PatternRelation {
    /// Return variables referenced by this relation.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Match(pattern) => state.terms.get(*pattern).referenced_variables(&state.terms),
            Self::Assign(pattern) => state.terms.get(*pattern).referenced_variables(state),
        }
    }
}

impl CheckState<'_> {
    /// Constrain one pattern against one value type.
    pub(in crate::check) fn constrain_pattern(
        &mut self,
        module: ModuleId,
        relation: PatternRelation,
        source: dir::LocalNodeIdAny,
        value: impl Into<TypeOperand>,
        condition: Condition,
    ) {
        let origin = Origin::Node(source.into_global(module));
        let constraint = Constraint::Pattern {
            relation,
            value: value.into(),
            origin,
            condition,
        };

        self.add_constraint(constraint);
    }
}
