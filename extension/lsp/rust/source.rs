//! Utilities for computing semantic tokens from parsed Dyst sources.

use std::cmp;

use dyst_language_source::{Source, Span};
use dyst_language_token::TokenSpan;
use tower_lsp_server::lsp_types as lsp;

/// Convert byte span to LSP range.
pub fn byte_span_to_range(source: &Source, span: Span) -> lsp::Range {
    let (start_line, start_column) = byte_to_utf16_position(source, span.start).unwrap_or_default();
    let (end_line, end_column) = byte_to_utf16_position(source, span.end).unwrap_or_default();
    lsp::Range {
        start: lsp::Position {
            line: start_line,
            character: start_column,
        },
        end: lsp::Position {
            line: end_line,
            character: end_column,
        },
    }
}

/// Convert LSP range to byte span in source.
pub fn range_to_byte_span(source: &Source, range: &lsp::Range) -> Option<(u32, u32)> {
    let start = position_to_byte(source, &range.start)?;
    let end = position_to_byte(source, &range.end)?;
    let start = cmp::min(start, source.len);
    let end = cmp::min(end, source.len);
    Some((cmp::min(start, end), cmp::max(start, end)))
}

/// Convert LSP position (line/character) to byte offset in source.
pub fn position_to_byte(source: &Source, position: &lsp::Position) -> Option<u32> {
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
pub fn byte_to_utf16_position(source: &Source, byte_index: u32) -> Option<(u32, u32)> {
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
pub fn token_length_utf16(source: &Source, token: &TokenSpan) -> u32 {
    let start = token.span.start as usize;
    let end = token.span.end as usize;
    source.content[start..end].encode_utf16().count() as u32
}
