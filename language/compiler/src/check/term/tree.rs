use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, GenericArgument, Reduction, TypeOperand, TypeTerm, VariableId};

/// Runtime tree expression term.
///
/// ```dsx
/// <Tag prop={value}>{child}</Tag>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TreeTerm {
    /// The source tree expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The explicit tag expression type, when any.
    pub(in crate::check) tag: Option<TypeOperand>,
    /// The explicit tag generic arguments.
    pub(in crate::check) generic_arguments: SmallVec<[GenericArgument; 2]>,
    /// The tree attribute argument types.
    pub(in crate::check) arguments: Vec<TypeOperand>,
    /// The child tree element argument types.
    pub(in crate::check) elements: Vec<TypeOperand>,
}

impl TreeTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 8]> {
        let mut variables = SmallVec::new();

        if let Some(tag) = self.tag {
            variables.extend(tag.referenced_variables(state));
        }
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| argument.referenced_variables(state)),
        );
        variables.extend(
            self.arguments
                .iter()
                .flat_map(|argument| argument.referenced_variables(state)),
        );
        variables.extend(
            self.elements
                .iter()
                .flat_map(|element| element.referenced_variables(state)),
        );

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
