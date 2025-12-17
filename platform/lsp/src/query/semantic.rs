use destack_source::File;
use destack_workspace::query;
use tower_lsp_server::lsp_types as lsp;

use super::common::byte_to_utf16_position;

/// Semantic token types in legend order (index = type id).
pub const SEMANTIC_TOKEN_TYPES: [lsp::SemanticTokenType; 22] = [
    lsp::SemanticTokenType::NAMESPACE,
    lsp::SemanticTokenType::TYPE,
    lsp::SemanticTokenType::CLASS,
    lsp::SemanticTokenType::ENUM,
    lsp::SemanticTokenType::INTERFACE,
    lsp::SemanticTokenType::STRUCT,
    lsp::SemanticTokenType::TYPE_PARAMETER,
    lsp::SemanticTokenType::PARAMETER,
    lsp::SemanticTokenType::VARIABLE,
    lsp::SemanticTokenType::PROPERTY,
    lsp::SemanticTokenType::ENUM_MEMBER,
    lsp::SemanticTokenType::FUNCTION,
    lsp::SemanticTokenType::METHOD,
    lsp::SemanticTokenType::MACRO,
    lsp::SemanticTokenType::KEYWORD,
    lsp::SemanticTokenType::MODIFIER,
    lsp::SemanticTokenType::COMMENT,
    lsp::SemanticTokenType::STRING,
    lsp::SemanticTokenType::NUMBER,
    lsp::SemanticTokenType::REGEXP,
    lsp::SemanticTokenType::OPERATOR,
    lsp::SemanticTokenType::DECORATOR,
];

/// Semantic token modifiers in legend order (bit index = modifier id).
pub const SEMANTIC_TOKEN_MODIFIERS: [lsp::SemanticTokenModifier; 10] = [
    lsp::SemanticTokenModifier::DECLARATION,
    lsp::SemanticTokenModifier::DEFINITION,
    lsp::SemanticTokenModifier::READONLY,
    lsp::SemanticTokenModifier::STATIC,
    lsp::SemanticTokenModifier::DEPRECATED,
    lsp::SemanticTokenModifier::ABSTRACT,
    lsp::SemanticTokenModifier::ASYNC,
    lsp::SemanticTokenModifier::MODIFICATION,
    lsp::SemanticTokenModifier::DOCUMENTATION,
    lsp::SemanticTokenModifier::DEFAULT_LIBRARY,
];

/// Build the legend advertised to the client.
pub fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
        token_modifiers: SEMANTIC_TOKEN_MODIFIERS.to_vec(),
    }
}

/// Convert a query SemanticTokenType to legend index.
fn token_type_to_index(token_type: query::SemanticTokenType) -> u32 {
    match token_type {
        query::SemanticTokenType::Namespace => 0,
        query::SemanticTokenType::Type => 1,
        query::SemanticTokenType::Class => 2,
        query::SemanticTokenType::Enum => 3,
        query::SemanticTokenType::Interface => 4,
        query::SemanticTokenType::Struct => 5,
        query::SemanticTokenType::TypeParameter => 6,
        query::SemanticTokenType::Parameter => 7,
        query::SemanticTokenType::Variable => 8,
        query::SemanticTokenType::Property => 9,
        query::SemanticTokenType::EnumMember => 10,
        query::SemanticTokenType::Function => 11,
        query::SemanticTokenType::Method => 12,
        query::SemanticTokenType::Macro => 13,
        query::SemanticTokenType::Keyword => 14,
        query::SemanticTokenType::Modifier => 15,
        query::SemanticTokenType::Comment => 16,
        query::SemanticTokenType::String => 17,
        query::SemanticTokenType::Number => 18,
        query::SemanticTokenType::Regexp => 19,
        query::SemanticTokenType::Operator => 20,
        query::SemanticTokenType::Decorator => 21,
        query::SemanticTokenType::Label => 8, // map to variable (no LSP Label type)
    }
}

/// Convert semantic tokens to delta-encoded LSP format.
pub fn tokens_to_lsp(file: &File, tokens: &[query::SemanticToken]) -> Vec<lsp::SemanticToken> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    for token in tokens {
        // convert byte offset to line/column
        let Some((line, character)) = byte_to_utf16_position(file, token.span.start) else {
            continue;
        };
        let Some((end_line, end_char)) = byte_to_utf16_position(file, token.span.end) else {
            continue;
        };

        // compute length (handle multi-line tokens)
        let length = if line == end_line {
            end_char - character
        } else {
            // for multi-line tokens, just use end of first line
            // this is a simplification; proper handling would split tokens
            end_char
        };

        // compute deltas
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            character - prev_char
        } else {
            character
        };

        result.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: token_type_to_index(token.token_type),
            token_modifiers_bitset: token.modifiers.bits(),
        });

        prev_line = line;
        prev_char = character;
    }

    result
}
