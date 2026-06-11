use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssignPatternTerm, CheckState, Condition, Constraint, ConstraintId, Origin, PatternTerm,
    TermId, TypeOperand,
};

/// Relation between a value type and a pattern.
///
/// Examples:
/// ```ds
/// match (value) { Some(item) => item }
/// const { name } = user
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum PatternRelation {
    /// Match pattern checks a value.
    ///
    /// Examples:
    /// ```ds
    /// match (value) { Some(item) => item }
    /// ```
    Match(TermId<PatternTerm>),
    /// Assignment pattern accepts an assigned value.
    ///
    /// Examples:
    /// ```ds
    /// const { name } = user
    /// ```
    Assign(TermId<AssignPatternTerm>),
}

impl CheckState<'_> {
    /// Relate one pattern to one value type.
    pub(in crate::check) fn constrain_pattern(
        &mut self,
        module: ModuleId,
        relation: PatternRelation,
        source: dir::LocalNodeIdAny,
        value: impl Into<TypeOperand>,
        condition: Condition,
    ) -> ConstraintId {
        let origin = Origin::Node(source.into_global(module));
        let constraint = Constraint::Pattern {
            relation,
            value: value.into(),
            origin,
            condition,
        };

        self.push_constraint(constraint)
    }
}
