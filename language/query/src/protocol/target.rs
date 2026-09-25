use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::{FileId, ModuleId, ProfileId, Span};

use crate::{QueryError, QueryResult};

/// One module in one program profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Module {
    /// The queried module.
    pub module_id: ModuleId,
    /// The queried profile.
    pub profile_id: ProfileId,
}

/// One byte position in a module source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct QueryPosition {
    /// The queried module.
    pub module: Module,
    /// The source file.
    pub file_id: FileId,
    /// The byte offset in the source file.
    pub offset: u32,
}

/// One source range in a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct QueryRange {
    /// The queried module.
    pub module: Module,
    /// The source range.
    pub span: Span,
}

/// One source target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Target {
    /// The target module.
    pub module: Module,
    /// The full source range.
    pub span: Span,
    /// The primary selection range.
    pub selection_span: Span,
}

impl Target {
    /// Create a source target whose full range is also its selection range.
    pub(crate) fn new(module: Module, span: Span) -> Self {
        Self {
            module,
            span,
            selection_span: span,
        }
    }

    /// Return this target with a selection span.
    pub(crate) fn with_selection_span(mut self, selection_span: Span) -> QueryResult<Self> {
        if !self.span.contains_span(selection_span) {
            return Err(QueryError::invalid(format!(
                "target selection {selection_span:?} outside {:?}",
                self.span
            )));
        }

        self.selection_span = selection_span;

        Ok(self)
    }
}
