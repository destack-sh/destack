use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::File;

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
pub const SEMANTIC_TOKEN_MODIFIERS: [lsp::SemanticTokenModifier; 11] = [
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
    lsp::SemanticTokenModifier::new("mutable"),
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
    // set up delta encoding state
    let mut result = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    // emit tokens in document order
    for token in tokens {
        // resolve token positions
        let Some((start_line, start_char)) = byte_to_utf16_position(file, token.span.start) else {
            continue;
        };
        let Some((end_line, end_char)) = byte_to_utf16_position(file, token.span.end) else {
            continue;
        };

        // cache type and modifiers for split segments
        let token_type = token_type_to_index(token.token_type);
        let modifiers = token.modifiers.bits();

        // emit single line tokens
        if start_line == end_line {
            let length = end_char.saturating_sub(start_char);
            if length == 0 {
                continue;
            }
            push_token(
                &mut result,
                &mut prev_line,
                &mut prev_char,
                start_line,
                start_char,
                length,
                token_type,
                modifiers,
            );
            continue;
        }

        // emit first line segment
        let Some(line_span) = file.get_line_span(start_line) else {
            continue;
        };
        if let Some(length) = utf16_len_between(file, token.span.start, line_span.end)
            && length > 0
        {
            push_token(
                &mut result,
                &mut prev_line,
                &mut prev_char,
                start_line,
                start_char,
                length,
                token_type,
                modifiers,
            );
        }

        // emit middle line segments
        for line in (start_line + 1)..end_line {
            let Some(line_span) = file.get_line_span(line) else {
                continue;
            };
            if let Some(length) = utf16_len_between(file, line_span.start, line_span.end)
                && length > 0
            {
                push_token(
                    &mut result,
                    &mut prev_line,
                    &mut prev_char,
                    line,
                    0,
                    length,
                    token_type,
                    modifiers,
                );
            }
        }

        // emit last line segment
        if end_char > 0 {
            push_token(
                &mut result,
                &mut prev_line,
                &mut prev_char,
                end_line,
                0,
                end_char,
                token_type,
                modifiers,
            );
        }
    }

    result
}

/// Convert a byte span to utf16 length.
fn utf16_len_between(file: &File, start: u32, end: u32) -> Option<u32> {
    if start > end || end > file.len {
        return None;
    }
    let slice = &file.text()[start as usize..end as usize];
    Some(slice.encode_utf16().count() as u32)
}

/// Push a token in delta encoded form.
#[allow(clippy::too_many_arguments)]
fn push_token(
    output: &mut Vec<lsp::SemanticToken>,
    prev_line: &mut u32,
    prev_char: &mut u32,
    line: u32,
    character: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
) {
    // compute delta encoding
    let delta_line = line.saturating_sub(*prev_line);
    let delta_start = if delta_line == 0 {
        character.saturating_sub(*prev_char)
    } else {
        character
    };

    output.push(lsp::SemanticToken {
        delta_line,
        delta_start,
        length,
        token_type,
        token_modifiers_bitset: modifiers,
    });

    // update previous position
    *prev_line = line;
    *prev_char = character;
}
