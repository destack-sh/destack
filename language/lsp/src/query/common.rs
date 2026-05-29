use std::borrow::Cow;
use std::cmp;

use destack_dir::TokenSpan;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::{File, Span};
use destack_workspace::{Repository, Revision};

use crate::uri::lsp_uri_for_file;

/// Convert byte span to LSP range.
pub fn byte_span_to_range(source: &File, span: Span) -> lsp::Range {
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

/// Convert LSP range to byte span in source.
pub fn range_to_byte_span(source: &File, range: &lsp::Range) -> Option<(u32, u32)> {
    let start = position_to_byte(source, &range.start)?;
    let end = position_to_byte(source, &range.end)?;
    let start = cmp::min(start, source.len);
    let end = cmp::min(end, source.len);
    Some((cmp::min(start, end), cmp::max(start, end)))
}

/// Convert LSP position (line/character) to byte offset in source.
pub fn position_to_byte(source: &File, position: &lsp::Position) -> Option<u32> {
    // always operate on a valid line start offset list
    let line_start_offsets = line_start_offsets_for_file(source);

    let line_index = position.line as usize;
    let line_start = *line_start_offsets.get(line_index).unwrap_or(&source.len);
    let next_start = line_start_offsets
        .get(line_index + 1)
        .copied()
        .unwrap_or(source.len);
    let slice = &source.text()[line_start as usize..next_start as usize];

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
pub fn byte_to_utf16_position(source: &File, byte_index: u32) -> Option<(u32, u32)> {
    Some(byte_to_utf16_position_clamped(source, byte_index))
}

/// Convert byte offset to LSP position with explicit end of file clamping.
fn byte_to_utf16_position_clamped(source: &File, byte_index: u32) -> (u32, u32) {
    // always operate on a valid line start offset list
    let line_start_offsets = line_start_offsets_for_file(source);

    // clamp out of bounds positions to end of file
    let byte_index = cmp::min(byte_index, source.len);

    // resolve the destination line for the clamped byte index
    let line_index = match line_start_offsets.binary_search(&byte_index) {
        Ok(idx) => idx as u32,
        Err(idx) => idx.saturating_sub(1) as u32,
    };
    let line_start = line_start_offsets[line_index as usize];
    let next_start = line_start_offsets
        .get(line_index as usize + 1)
        .copied()
        .unwrap_or(source.len);
    let slice = &source.text()[line_start as usize..next_start as usize];

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

    (line_index, utf16_column)
}

/// Return line-start offsets for one file, computing them when missing.
fn line_start_offsets_for_file(source: &File) -> Cow<'_, [u32]> {
    // reuse precomputed offsets when present
    if let Some(line_start_offsets) = source.line_start_offsets() {
        return Cow::Borrowed(line_start_offsets);
    }

    // derive offsets from text for unloaded or ad hoc snapshots
    let mut line_start_offsets = vec![0];
    for (offset, ch) in source.text().char_indices() {
        if ch == '\n' {
            line_start_offsets.push(offset as u32 + 1);
        }
    }

    Cow::Owned(line_start_offsets)
}

/// Compute UTF-16 length of a token span.
pub fn token_length_utf16(source: &File, token: &TokenSpan) -> u32 {
    let start = token.span.start as usize;
    let end = token.span.end as usize;
    source.text()[start..end].encode_utf16().count() as u32
}

/// Convert a span to an LSP location.
pub fn span_to_location(
    repository: &Repository,
    revision: Revision,
    span: Span,
) -> Option<lsp::Location> {
    let file = repository.file(revision, span.file).ok().flatten()?;
    let range = byte_span_to_range(&file, span);
    let uri = lsp_uri_for_file(&file)?;
    Some(lsp::Location { uri, range })
}

/// Convert query symbol kind to LSP symbol kind.
pub fn symbol_kind_to_lsp(kind: query::SymbolKind) -> lsp::SymbolKind {
    match kind {
        query::SymbolKind::File => lsp::SymbolKind::FILE,
        query::SymbolKind::Module => lsp::SymbolKind::MODULE,
        query::SymbolKind::Namespace => lsp::SymbolKind::NAMESPACE,
        query::SymbolKind::Package => lsp::SymbolKind::PACKAGE,
        query::SymbolKind::Class => lsp::SymbolKind::CLASS,
        query::SymbolKind::Method => lsp::SymbolKind::METHOD,
        query::SymbolKind::Property => lsp::SymbolKind::PROPERTY,
        query::SymbolKind::Field => lsp::SymbolKind::FIELD,
        query::SymbolKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
        query::SymbolKind::Enum => lsp::SymbolKind::ENUM,
        query::SymbolKind::Interface => lsp::SymbolKind::INTERFACE,
        query::SymbolKind::Function => lsp::SymbolKind::FUNCTION,
        query::SymbolKind::Variable => lsp::SymbolKind::VARIABLE,
        query::SymbolKind::Constant => lsp::SymbolKind::CONSTANT,
        query::SymbolKind::String => lsp::SymbolKind::STRING,
        query::SymbolKind::Number => lsp::SymbolKind::NUMBER,
        query::SymbolKind::Boolean => lsp::SymbolKind::BOOLEAN,
        query::SymbolKind::Array => lsp::SymbolKind::ARRAY,
        query::SymbolKind::Object => lsp::SymbolKind::OBJECT,
        query::SymbolKind::Key => lsp::SymbolKind::KEY,
        query::SymbolKind::Null => lsp::SymbolKind::NULL,
        query::SymbolKind::EnumMember => lsp::SymbolKind::ENUM_MEMBER,
        query::SymbolKind::Struct => lsp::SymbolKind::STRUCT,
        query::SymbolKind::Event => lsp::SymbolKind::EVENT,
        query::SymbolKind::Operator => lsp::SymbolKind::OPERATOR,
        query::SymbolKind::TypeParameter => lsp::SymbolKind::TYPE_PARAMETER,
    }
}
