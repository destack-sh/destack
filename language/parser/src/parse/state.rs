use destack_ast::StringId;

use super::parser::TypeLiteralIdentifiers;

/// Semantic parser bookkeeping that is separate from token transport.
#[derive(Debug)]
pub(crate) struct ParserState {
    /// Expression recursion depth for periodic stack growth checks.
    pub(crate) expression_stack_depth: u32,
    /// Statement recursion depth for periodic stack growth checks.
    pub(crate) statement_stack_depth: u32,
    /// Cached identifier lookup for identifier tokens.
    pub(crate) token_identifiers: Vec<Option<StringId>>,
    /// Cached-state bits for identifier lookup entries.
    pub(crate) token_identifiers_cached: Vec<bool>,
    /// Cached identifiers used by type literal parsing.
    pub(crate) type_literal_identifiers: TypeLiteralIdentifiers,
}

impl ParserState {
    /// Create parser semantic state sized for the current file.
    pub(crate) fn new(
        estimated_tokens: usize,
        type_literal_identifiers: TypeLiteralIdentifiers,
    ) -> Self {
        Self {
            expression_stack_depth: 0,
            statement_stack_depth: 0,
            token_identifiers: Vec::with_capacity(estimated_tokens),
            token_identifiers_cached: Vec::with_capacity(estimated_tokens),
            type_literal_identifiers,
        }
    }

    /// Reset semantic parser state for a fresh parse.
    pub(crate) fn reset(&mut self) {
        self.expression_stack_depth = 0;
        self.statement_stack_depth = 0;
    }

    /// Truncate identifier caches to match the current token count.
    pub(crate) fn truncate_identifier_caches(&mut self, len: usize) {
        self.token_identifiers.truncate(len);
        self.token_identifiers_cached.truncate(len);
    }

    /// Ensure identifier caches can index at least `index`.
    pub(crate) fn ensure_identifier_cache_capacity(&mut self, index: usize) {
        if self.token_identifiers.len() > index {
            return;
        }

        let required_len = index + 1;
        let grown_len = self
            .token_identifiers
            .len()
            .saturating_add(self.token_identifiers.len() / 2)
            .saturating_add(64);
        let new_len = required_len.max(grown_len);
        self.token_identifiers.resize(new_len, None);
        self.token_identifiers_cached.resize(new_len, false);
    }
}
