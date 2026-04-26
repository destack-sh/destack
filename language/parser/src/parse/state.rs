use super::parser::TypeLiteralIdentifiers;

/// Semantic parser bookkeeping that is separate from token transport.
#[derive(Debug)]
pub(crate) struct ParserState {
    /// Expression recursion depth for periodic stack growth checks.
    pub(crate) expression_stack_depth: u32,
    /// Statement recursion depth for periodic stack growth checks.
    pub(crate) statement_stack_depth: u32,
    /// Cached identifiers used by type literal parsing.
    pub(crate) type_literal_identifiers: TypeLiteralIdentifiers,
}

impl ParserState {
    /// Create parser semantic state sized for the current file.
    pub(crate) fn new(type_literal_identifiers: TypeLiteralIdentifiers) -> Self {
        Self {
            expression_stack_depth: 0,
            statement_stack_depth: 0,
            type_literal_identifiers,
        }
    }

    /// Reset semantic parser state for a fresh parse.
    pub(crate) fn reset(&mut self) {
        self.expression_stack_depth = 0;
        self.statement_stack_depth = 0;
    }
}
