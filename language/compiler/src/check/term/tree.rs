use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, GenericArgument, TermId, TypeOperand};
use crate::{CompilerError, CompilerResult};

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

impl CheckState<'_> {
    /// Reduce one tree expression when tree dispatch is known.
    pub(in crate::check) fn reduce_tree_term(
        &mut self,
        tree: TermId<TreeTerm>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let source = self.inference.term(tree).source;

        Err(CompilerError::Internal {
            message: format!("tree expression {source:?} cannot be reduced yet"),
        })
    }
}
