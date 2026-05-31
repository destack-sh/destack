use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssignPatternTerm, CheckState, Condition, Constraint, Origin, PatternTerm, TermId, TypeOperand,
};

/// Relation between a value type and a pattern.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternRelation {
    /// Match pattern checks a value.
    Match(TermId<PatternTerm>),
    /// Assignment pattern accepts an assigned value.
    Assign(TermId<AssignPatternTerm>),
}

impl CheckState<'_> {
    /// Relate one pattern to one value type.
    pub(in crate::check) fn relate_pattern(
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

        self.push_constraint(constraint);
    }
}
