use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    AssignPatternTerm, CheckState, Constraint, ConstraintOrigin, PatternTerm, TermId, TermTable,
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
        terms: &TermTable,
    ) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Match(pattern) => terms.get(*pattern).referenced_variables(terms),
            Self::Assign(pattern) => terms.get(*pattern).referenced_variables(terms),
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
        value: VariableId,
    ) {
        let origin = ConstraintOrigin::Node(source.into_global(module));
        let constraint = Constraint::Pattern {
            relation,
            value,
            origin,
            condition: self.active_static_condition(module),
        };

        self.add_constraint(constraint);
    }
}
