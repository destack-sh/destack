use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_source::{File, Span};

use super::error::{internal_error, workspace_error};
use super::source::SourceFiles;
use crate::uri;

/// Convert one exact byte span to an LSP range.
pub(super) fn range(source: &File, span: Span) -> jsonrpc::Result<lsp::Range> {
    if span.file != source.id {
        return Err(internal_error(format!(
            "span {:?} does not belong to source file {:?}",
            span, source.id
        )));
    }
    if span.start > span.end {
        return Err(internal_error(format!("span is reversed: {span:?}")));
    }

    let start = position(source, span.start)?;
    let end = position(source, span.end)?;

    Ok(lsp::Range { start, end })
}

/// Convert an LSP position to an exact byte offset in source.
pub(super) fn offset(source: &File, position: &lsp::Position) -> jsonrpc::Result<u32> {
    let line_start_offsets = source.line_start_offsets().ok_or_else(|| {
        jsonrpc::Error::invalid_params(format!("source file {:?} has no line index", source.id))
    })?;

    // resolve the requested source line
    let line_index = position.line as usize;
    let line_start = *line_start_offsets.get(line_index).ok_or_else(|| {
        jsonrpc::Error::invalid_params(format!(
            "line {} is outside source file {:?}",
            position.line, source.id
        ))
    })?;
    let next_line_start = line_start_offsets.get(line_index + 1).copied();
    let line_end = next_line_start
        .map(|offset| offset - 1)
        .unwrap_or(source.len);
    let line = source
        .text()
        .get(line_start as usize..line_end as usize)
        .ok_or_else(|| {
            internal_error(format!(
                "line {} has an invalid byte range in source file {:?}",
                position.line, source.id
            ))
        })?;

    // resolve the requested UTF-16 column
    let mut utf16_column = 0u32;
    let mut byte_column = 0u32;
    for character in line.chars() {
        if utf16_column == position.character {
            break;
        }

        let character_width = character.len_utf16() as u32;
        if utf16_column + character_width > position.character {
            return Err(jsonrpc::Error::invalid_params(format!(
                "character {} splits a UTF-16 surrogate pair on line {}",
                position.character, position.line
            )));
        }

        utf16_column += character_width;
        byte_column += character.len_utf8() as u32;
    }

    if utf16_column != position.character {
        return Err(jsonrpc::Error::invalid_params(format!(
            "character {} is outside line {}",
            position.character, position.line
        )));
    }

    Ok(line_start + byte_column)
}

/// Convert one exact byte offset to an LSP position.
pub(super) fn position(source: &File, byte_index: u32) -> jsonrpc::Result<lsp::Position> {
    let Some((line, byte_column)) = source.get_position(byte_index) else {
        return Err(internal_error(format!(
            "byte offset {byte_index} is outside source file {:?} with length {}",
            source.id, source.len
        )));
    };
    let line_start = byte_index - byte_column;
    let Some(prefix) = source.text().get(line_start as usize..byte_index as usize) else {
        return Err(internal_error(format!(
            "byte offset {byte_index} is not a character boundary in source file {:?}",
            source.id
        )));
    };
    let character = prefix.encode_utf16().count() as u32;

    Ok(lsp::Position { line, character })
}

/// Convert a span to an LSP location.
pub(super) fn location(span: Span, files: &SourceFiles) -> jsonrpc::Result<lsp::Location> {
    let file = files.file(span.file).map_err(workspace_error)?;
    let range = range(&file, span)?;
    let uri = uri::file(&file)?;

    Ok(lsp::Location { uri, range })
}
