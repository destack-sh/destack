use std::borrow::Cow;
use std::cmp;
use std::sync::Arc;

use destack_lsp_types as lsp;
use destack_source::{File, FileId, Span};

use crate::uri::lsp_uri_for_file;

/// Convert byte span to LSP range.
pub(super) fn byte_span_to_range(source: &File, span: Span) -> lsp::Range {
    let (start_line, start_column) = byte_to_utf16_position_clamped(source, span.start);
    let (end_line, end_column) = byte_to_utf16_position_clamped(source, span.end);

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

/// Convert LSP position to byte offset in source.
pub(super) fn position_to_byte(source: &File, position: &lsp::Position) -> Option<u32> {
    let line_start_offsets = line_start_offsets_for_file(source);

    // resolve the requested line range
    let line_index = position.line as usize;
    let line_start = *line_start_offsets.get(line_index).unwrap_or(&source.len);
    let next_start = line_start_offsets
        .get(line_index + 1)
        .copied()
        .unwrap_or(source.len);
    let slice = &source.text()[line_start as usize..next_start as usize];

    // walk UTF-16 units to the requested character
    let mut utf16_units = 0u32;
    let mut byte_offset = 0usize;
    for character in slice.chars() {
        if utf16_units >= position.character {
            break;
        }

        let character_units = character.len_utf16() as u32;
        if utf16_units + character_units > position.character {
            break;
        }

        utf16_units += character_units;
        byte_offset += character.len_utf8();
    }

    // clamp columns past the line end
    if utf16_units < position.character {
        return Some(next_start);
    }

    Some(line_start + byte_offset as u32)
}

/// Convert byte offset to LSP position.
pub(super) fn byte_to_utf16_position(source: &File, byte_index: u32) -> (u32, u32) {
    byte_to_utf16_position_clamped(source, byte_index)
}

/// Convert a span to an LSP location.
pub(super) fn span_to_location(
    span: Span,
    file_for_id: &mut impl FnMut(FileId) -> Option<Arc<File>>,
) -> Option<lsp::Location> {
    let file = file_for_id(span.file)?;
    let range = byte_span_to_range(&file, span);
    let uri = lsp_uri_for_file(&file)?;

    Some(lsp::Location { uri, range })
}

/// Convert byte offset to LSP position with explicit end-of-file clamping.
fn byte_to_utf16_position_clamped(source: &File, byte_index: u32) -> (u32, u32) {
    let line_start_offsets = line_start_offsets_for_file(source);

    // clamp out-of-bounds positions to end of file
    let byte_index = cmp::min(byte_index, source.len);

    // resolve the destination line for the clamped byte index
    let line_index = match line_start_offsets.binary_search(&byte_index) {
        Ok(index) => index as u32,
        Err(index) => index.saturating_sub(1) as u32,
    };
    let line_start = line_start_offsets[line_index as usize];
    let next_start = line_start_offsets
        .get(line_index as usize + 1)
        .copied()
        .unwrap_or(source.len);
    let slice = &source.text()[line_start as usize..next_start as usize];

    // walk UTF-16 units to the target byte
    let target_offset = (byte_index - line_start) as usize;
    let mut consumed = 0usize;
    let mut utf16_column = 0u32;
    for character in slice.chars() {
        if consumed >= target_offset {
            break;
        }

        let length = character.len_utf8();
        if consumed + length > target_offset {
            break;
        }

        consumed += length;
        utf16_column += character.len_utf16() as u32;
    }

    (line_index, utf16_column)
}

/// Return line-start offsets for one file, computing them when missing.
fn line_start_offsets_for_file(source: &File) -> Cow<'_, [u32]> {
    if let Some(line_start_offsets) = source.line_start_offsets() {
        return Cow::Borrowed(line_start_offsets);
    }

    // derive offsets from text for loaded editor images
    let mut line_start_offsets = vec![0];
    for (offset, character) in source.text().char_indices() {
        if character == '\n' {
            line_start_offsets.push(offset as u32 + 1);
        }
    }

    Cow::Owned(line_start_offsets)
}
