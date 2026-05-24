use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{ArgumentTerm, VariableId};

/// Runtime template string term.
///
/// ```ts
/// `/${prefix}/${id}`
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TemplateTerm {
    /// The source template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The literal string segments.
    pub(in crate::check) strings: Vec<dir::StringId>,
    /// The interpolated expression types.
    pub(in crate::check) spans: Vec<VariableId>,
}

impl TemplateTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        self.spans.iter().copied().collect()
    }
}

/// Runtime tagged template term.
///
/// ```ts
/// sql<User>`select ${id}`
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TaggedTemplateTerm {
    /// The source tagged template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tag expression type.
    pub(in crate::check) tag: VariableId,
    /// The explicit tag generic arguments.
    pub(in crate::check) generic_arguments: Vec<ArgumentTerm>,
    /// The literal string segments.
    pub(in crate::check) strings: Vec<dir::StringId>,
    /// The interpolated expression types.
    pub(in crate::check) spans: Vec<VariableId>,
}

impl TaggedTemplateTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.push(self.tag);
        variables.extend(self.generic_arguments.iter().map(ArgumentTerm::variable));
        variables.extend(self.spans.iter().copied());

        variables
    }
}
