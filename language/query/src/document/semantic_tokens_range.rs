use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    ModuleQueryContext, ProgramQueryContext, QueryRange, QueryResult, SemanticToken,
    SemanticTokensRequest,
};

/// A semantic tokens range request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensRangeRequest {
    /// The queried range.
    pub range: QueryRange,
}

/// A semantic tokens range response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensRangeResponse {
    /// Semantic tokens.
    pub tokens: Vec<SemanticToken>,
}

impl ModuleQueryContext<'_> {
    /// Return semantic tokens wholly contained by a source range.
    pub fn semantic_tokens_range(
        &self,
        request: SemanticTokensRangeRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<SemanticTokensRangeResponse> {
        let range = request.range;
        let response = self.semantic_tokens(
            SemanticTokensRequest {
                module: range.module,
                file_id: range.span.file,
            },
            program,
        )?;
        let tokens = response
            .tokens
            .into_iter()
            .filter(|token| {
                token.span.start >= range.span.start && token.span.end <= range.span.end
            })
            .collect();

        Ok(SemanticTokensRangeResponse { tokens })
    }
}
