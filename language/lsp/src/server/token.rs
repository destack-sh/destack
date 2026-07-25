use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::File;

use super::error::internal_error;
use super::position;

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
pub(super) fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
        token_modifiers: SEMANTIC_TOKEN_MODIFIERS.to_vec(),
    }
}

/// Return the legend index for a semantic token type.
fn type_index(token_type: query::SemanticTokenType) -> u32 {
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
        // map labels to variables because LSP has no label token type
        query::SemanticTokenType::Label => 8,
    }
}

/// Convert semantic tokens to delta-encoded LSP format.
pub(super) fn encode(
    file: &File,
    tokens: &[query::SemanticToken],
) -> jsonrpc::Result<Vec<lsp::SemanticToken>> {
    // set up delta encoding state
    let mut result = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    // emit tokens in document order
    for token in tokens {
        if token.span.file != file.id {
            return Err(internal_error(format!(
                "semantic token {:?} does not belong to source file {:?}",
                token.span, file.id
            )));
        }

        // resolve token positions
        let start = position::position(file, token.span.start)?;
        let end = position::position(file, token.span.end)?;

        // cache type and modifiers for split segments
        let token_type = type_index(token.token_type);
        let modifiers = token.modifiers.bits();

        // emit single line tokens
        if start.line == end.line {
            let Some(length) = end.character.checked_sub(start.character) else {
                return Err(internal_error(format!(
                    "semantic token has reversed span: {:?}",
                    token.span
                )));
            };
            if length == 0 {
                return Err(internal_error(format!(
                    "semantic token has an empty span: {:?}",
                    token.span
                )));
            }
            push_token(
                &mut result,
                &mut prev_line,
                &mut prev_char,
                start.line,
                start.character,
                length,
                token_type,
                modifiers,
            )?;
            continue;
        }

        // emit first line segment
        let line_span = file.get_line_span(start.line).ok_or_else(|| {
            internal_error(format!(
                "semantic token starts outside source file: {:?}",
                token.span
            ))
        })?;
        let length = utf16_len_between(file, token.span.start, line_span.end)?;
        if length > 0 {
            push_token(
                &mut result,
                &mut prev_line,
                &mut prev_char,
                start.line,
                start.character,
                length,
                token_type,
                modifiers,
            )?;
        }

        // emit middle line segments
        for line in (start.line + 1)..end.line {
            let line_span = file.get_line_span(line).ok_or_else(|| {
                internal_error(format!(
                    "semantic token crosses a missing source line: {:?}",
                    token.span
                ))
            })?;
            let length = utf16_len_between(file, line_span.start, line_span.end)?;
            if length > 0 {
                push_token(
                    &mut result,
                    &mut prev_line,
                    &mut prev_char,
                    line,
                    0,
                    length,
                    token_type,
                    modifiers,
                )?;
            }
        }

        // emit last line segment
        if end.character > 0 {
            push_token(
                &mut result,
                &mut prev_line,
                &mut prev_char,
                end.line,
                0,
                end.character,
                token_type,
                modifiers,
            )?;
        }
    }

    Ok(result)
}

/// Convert a byte span to utf16 length.
fn utf16_len_between(file: &File, start: u32, end: u32) -> jsonrpc::Result<u32> {
    if start > end || end > file.len {
        return Err(internal_error(format!(
            "source range {start}..{end} is outside file {:?} with length {}",
            file.id, file.len
        )));
    }
    let Some(slice) = file.text().get(start as usize..end as usize) else {
        return Err(internal_error(format!(
            "source range {start}..{end} is not on character boundaries in file {:?}",
            file.id
        )));
    };

    Ok(slice.encode_utf16().count() as u32)
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
) -> jsonrpc::Result<()> {
    // compute delta encoding
    let Some(delta_line) = line.checked_sub(*prev_line) else {
        return Err(internal_error("semantic tokens are not in source order"));
    };
    let delta_start = if delta_line == 0 {
        character
            .checked_sub(*prev_char)
            .ok_or_else(|| internal_error("semantic tokens are not in source order"))?
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

    Ok(())
}
