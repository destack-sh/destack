use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::{ModuleQueryContext, ProgramQueryContext, QueryRange, QueryResult, SemanticToken};

/// Request semantic tokens for a document range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensRangeRequest {
    /// The queried range.
    pub range: QueryRange,
}

/// Response payload for range semantic token queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensRangeResponse {
    /// Semantic tokens.
    pub tokens: Vec<SemanticToken>,
}

impl ModuleQueryContext<'_> {
    /// Return semantic tokens wholly contained by a source range.
    pub fn semantic_tokens_range(
        &self,
        program: &ProgramQueryContext<'_>,
        range: Span,
    ) -> QueryResult<Vec<SemanticToken>> {
        let tokens = self.semantic_tokens(program, range.file)?;
        let tokens = tokens
            .into_iter()
            .filter(|token| token.span.start >= range.start && token.span.end <= range.end)
            .collect();

        Ok(tokens)
    }
}
