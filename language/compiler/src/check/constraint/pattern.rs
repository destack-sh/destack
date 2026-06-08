use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    AssignPatternTerm, CheckState, Condition, Constraint, Origin, PatternTerm, TermId, TypeOperand,
    VariableId,
};

/// Relation between a value type and a pattern.
///
/// Examples:
/// ```ds
/// match (value) { Some(item) => item }
/// const { name } = user
/// ```
#[derive(Debug, Clone, PartialEq)]
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

impl PatternRelation {
    /// Return variables referenced by this pattern relation.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        match self {
            Self::Match(pattern) => state.inference.term(*pattern).referenced_variables(state),
            Self::Assign(pattern) => state.inference.term(*pattern).referenced_variables(state),
        }
    }
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
