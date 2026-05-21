use super::identifier::TypeLiteralIdentifiers;

/// Semantic parser bookkeeping that is separate from token transport.
#[derive(Debug)]
pub(crate) struct ParserState {
    /// Cached identifiers used by type literal parsing.
    pub(crate) type_literal_identifiers: TypeLiteralIdentifiers,
}

impl ParserState {
    /// Create parser semantic state sized for the current file.
    pub(crate) fn new(type_literal_identifiers: TypeLiteralIdentifiers) -> Self {
        Self {
            type_literal_identifiers,
        }
    }

    /// Reset semantic parser state for a fresh parse.
    pub(crate) fn reset(&mut self) {}
}
