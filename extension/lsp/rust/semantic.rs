//! Utilities for computing semantic tokens from parsed Dyst sources.

use std::cmp;

use dyst_language_ast::{Module, NodeId, NodeTree, NodeVisitor, SemanticTokenMap, SemanticType};
use dyst_language_source::Source;
use dyst_language_token::TokenSpan;
use tower_lsp_server::lsp_types as lsp;

/// All semantic token types supported by the LSP server.
pub const SEMANTIC_TOKEN_TYPES: [lsp::SemanticTokenType; 23] = [
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
    lsp::SemanticTokenType::EVENT,
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

/// Build the legend advertised to the client.
pub fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
        token_modifiers: Vec::new(),
    }
}

/// Compute all semantic tokens for the given document.
pub fn collect_full_tokens(
    source: &Source,
    tokens: &Vec<TokenSpan>,
    tree: Option<(&NodeTree, NodeId<Module>)>,
) -> Option<Vec<lsp::SemanticToken>> {
    collect_tokens(source, tokens, tree, None)
}

/// Compute semantic tokens restricted to a range.
pub fn collect_range_tokens(
    source: &Source,
    tokens: &Vec<TokenSpan>,
    tree: Option<(&NodeTree, NodeId<Module>)>,
    range: &lsp::Range,
) -> Option<Vec<lsp::SemanticToken>> {
    collect_tokens(source, tokens, tree, Some(range))
}

/// Collect semantic tokens from source, optionally filtered by range.
fn collect_tokens(
    source: &Source,
    tokens: &Vec<TokenSpan>,
    tree: Option<(&NodeTree, NodeId<Module>)>,
    range: Option<&lsp::Range>,
) -> Option<Vec<lsp::SemanticToken>> {
    if tokens.is_empty() {
        return Some(Vec::new());
    }

    // build semantic type mapping from AST if available
    let mut semantic_map = SemanticTokenMap::from_tokens(source, tokens);
    if let Some((tree, module_id)) = tree {
        let module = tree.get(module_id);
        semantic_map.visit_module(tree, module_id, module);
    }

    // convert range to byte span for filtering
    let byte_span = if let Some(range) = range {
        match range_to_byte_span(source, range) {
            Some(span) => Some(span),
            None => return None,
        }
    } else {
        None
    };

    encode_tokens(source, tokens, &semantic_map.semantic_types, byte_span)
}

/// Encode tokens into LSP semantic token format with delta encoding.
fn encode_tokens(
    source: &Source,
    tokens: &[TokenSpan],
    semantic_types: &[SemanticType],
    byte_span: Option<(u32, u32)>,
) -> Option<Vec<lsp::SemanticToken>> {
    debug_assert_eq!(tokens.len(), semantic_types.len());

    let mut encoded: Vec<lsp::SemanticToken> = Vec::new();
    let mut previous_line = 0u32;
    let mut previous_column = 0u32;
    let mut is_first = true;

    for (token, semantic) in tokens.iter().zip(semantic_types.iter()) {
        let mapped_type = match type_index(*semantic) {
            Some(index) => index,
            None => continue,
        };

        // skip tokens outside the requested range
        if let Some((start, end)) = byte_span
            && (token.span.end <= start || token.span.start >= end)
        {
            continue;
        }

        let (line, column) = byte_to_utf16_position(source, token.span.start)?;
        let length = token_length_utf16(source, token);

        // compute deltas for LSP encoding
        let delta_line = if is_first {
            line
        } else {
            line.saturating_sub(previous_line)
        };
        let delta_start = if is_first || delta_line > 0 {
            column
        } else {
            column.saturating_sub(previous_column)
        };

        encoded.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: mapped_type,
            token_modifiers_bitset: 0,
        });

        previous_line = line;
        previous_column = column;
        is_first = false;
    }

    Some(encoded)
}

/// Convert LSP range to byte span in source.
fn range_to_byte_span(source: &Source, range: &lsp::Range) -> Option<(u32, u32)> {
    let start = position_to_byte(source, &range.start)?;
    let end = position_to_byte(source, &range.end)?;
    let start = cmp::min(start, source.len);
    let end = cmp::min(end, source.len);
    Some((cmp::min(start, end), cmp::max(start, end)))
}

/// Convert LSP position (line/character) to byte offset in source.
fn position_to_byte(source: &Source, position: &lsp::Position) -> Option<u32> {
    let line_index = position.line as usize;
    let line_start = *source
        .line_start_offsets
        .get(line_index)
        .unwrap_or(&source.len);
    let next_start = source
        .line_start_offsets
        .get(line_index + 1)
        .copied()
        .unwrap_or(source.len);
    let slice = &source.content[line_start as usize..next_start as usize];

    // walk characters counting UTF-16 units until we reach target
    let mut utf16_units = 0u32;
    let mut byte_offset = 0usize;
    for ch in slice.chars() {
        if utf16_units >= position.character {
            break;
        }
        let ch_units = ch.len_utf16() as u32;
        if utf16_units + ch_units > position.character {
            break;
        }
        utf16_units += ch_units;
        byte_offset += ch.len_utf8();
    }

    // if we didn't reach the target character, clamp to end of line
    if utf16_units < position.character {
        return Some(next_start);
    }

    Some(line_start + byte_offset as u32)
}

/// Convert byte offset to LSP position (line/character in UTF-16).
fn byte_to_utf16_position(source: &Source, byte_index: u32) -> Option<(u32, u32)> {
    if byte_index > source.len {
        return None;
    }

    // find which line contains this byte offset
    let line_index = match source.line_start_offsets.binary_search(&byte_index) {
        Ok(idx) => idx as u32,
        Err(idx) => idx.saturating_sub(1) as u32,
    };

    let line_start = source.line_start_offsets[line_index as usize];
    let next_start = source
        .line_start_offsets
        .get(line_index as usize + 1)
        .copied()
        .unwrap_or(source.len);
    let slice = &source.content[line_start as usize..next_start as usize];

    // walk characters counting UTF-16 units until we reach target byte
    let target_offset = (byte_index - line_start) as usize;
    let mut consumed = 0usize;
    let mut utf16_column = 0u32;
    for ch in slice.chars() {
        if consumed >= target_offset {
            break;
        }
        let len = ch.len_utf8();
        if consumed + len > target_offset {
            break;
        }
        consumed += len;
        utf16_column += ch.len_utf16() as u32;
    }

    Some((line_index, utf16_column))
}

/// Compute UTF-16 length of a token span.
fn token_length_utf16(source: &Source, token: &TokenSpan) -> u32 {
    let start = token.span.start as usize;
    let end = token.span.end as usize;
    source.content[start..end].encode_utf16().count() as u32
}

/// Get the index of a SemanticType in our token types array.
fn type_index(semantic_type: SemanticType) -> Option<u32> {
    let lsp_type = get_lsp_semantic_type(semantic_type);
    SEMANTIC_TOKEN_TYPES
        .iter()
        .position(|candidate| candidate == &lsp_type)
        .map(|idx| idx as u32)
}

/// Map a SemanticType to the corresponding LSP token type.
pub(crate) fn get_lsp_semantic_type(semantic_type: SemanticType) -> lsp::SemanticTokenType {
    match semantic_type {
        SemanticType::Keyword => lsp::SemanticTokenType::KEYWORD,
        SemanticType::Identifier => lsp::SemanticTokenType::VARIABLE,
        SemanticType::LiteralNumbery => lsp::SemanticTokenType::NUMBER,
        SemanticType::LiteralStringy => lsp::SemanticTokenType::STRING,
        SemanticType::Operator => lsp::SemanticTokenType::OPERATOR,
        SemanticType::Whitespace => lsp::SemanticTokenType::OPERATOR,
        SemanticType::Parenthesis => lsp::SemanticTokenType::OPERATOR,
        SemanticType::Symbol => lsp::SemanticTokenType::OPERATOR,
        SemanticType::Doc => lsp::SemanticTokenType::COMMENT,
        SemanticType::Comment => lsp::SemanticTokenType::COMMENT,
        SemanticType::Modifier => lsp::SemanticTokenType::MODIFIER,
        SemanticType::Macro => lsp::SemanticTokenType::MACRO,
        SemanticType::Type => lsp::SemanticTokenType::TYPE,
        SemanticType::Function => lsp::SemanticTokenType::FUNCTION,
        SemanticType::Parameter => lsp::SemanticTokenType::PARAMETER,
        SemanticType::Argument => lsp::SemanticTokenType::PARAMETER,
        SemanticType::Variable => lsp::SemanticTokenType::VARIABLE,
    }
}
