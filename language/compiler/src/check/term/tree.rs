use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{ArgumentTerm, CheckState, Reduction, TermId, TypeTerm, VariableId};

/// Runtime tree expression term.
///
/// ```tsx
/// <Tag prop={value}>{child}</Tag>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TreeTerm {
    /// The source tree expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The explicit tag expression type, when any.
    pub(in crate::check) tag: Option<VariableId>,
    /// The explicit tag generic arguments.
    pub(in crate::check) generic_arguments: Vec<TermId<ArgumentTerm>>,
    /// The tree attribute argument types.
    pub(in crate::check) arguments: Vec<VariableId>,
    /// The child tree element argument types.
    pub(in crate::check) elements: Vec<VariableId>,
}

impl TreeTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 8]> {
        let mut variables = SmallVec::new();

        variables.extend(self.tag);
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| state.argument_variables(*argument)),
        );
        variables.extend(self.arguments.iter().copied());
        variables.extend(self.elements.iter().copied());

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one tree expression when tree dispatch is known.
    pub(in crate::check) fn reduce_tree_term(
        &mut self,
        _module: ModuleId,
        _tree: &TreeTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        Ok(Reduction::pending())
    }
}
