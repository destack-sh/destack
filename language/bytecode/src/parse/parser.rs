use std::collections::HashMap;
use std::fmt;

use tspp_source::{FileId, Span};

use crate::{Function, FunctionId, ObjectBuilder};

use super::cursor::TokenCursor;

/// Parser over one tokenized bytecode object.
pub struct Parser<'source> {
    /// Tokenized source cursor.
    pub(super) cursor: TokenCursor<'source>,
    /// Bytecode object being built.
    pub(super) object: ObjectBuilder,
    /// Dense function ids keyed by source name.
    pub(super) function_ids: HashMap<String, FunctionId>,
    /// Source names in dense function order.
    pub(super) function_names: Vec<String>,
    /// Object-local functions in dense identity order.
    pub(super) functions: Vec<Function>,
}

impl fmt::Debug for Parser<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Parser").finish_non_exhaustive()
    }
}

impl<'source> Parser<'source> {
    /// Create one parser.
    pub fn new(file_id: FileId, source: &'source str) -> Self {
        Self {
            cursor: TokenCursor::new(file_id, source),
            object: ObjectBuilder::new(),
            function_ids: HashMap::new(),
            function_names: Vec::new(),
            functions: Vec::new(),
        }
    }

    /// Return source names in dense function order.
    pub fn function_names(&self) -> &[String] {
        &self.function_names
    }

    /// Return an empty span in this parser's source file.
    pub(super) fn empty_span(&self) -> Span {
        Span::empty(self.cursor.file_id)
    }
}
