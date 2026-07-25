use destack_formatter::{format_source, format_source_range};
use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_repository::FormatterOptions;
use destack_source::{DiagnosticSeverity, File, Span};

use super::error::internal_error;
use super::position;

/// Format one file image and return a document edit.
pub(super) fn file_edit(
    file: &File,
    formatter: FormatterOptions,
) -> jsonrpc::Result<Option<lsp::TextEdit>> {
    let formatted = format_source(file, file.text(), formatter).map_err(internal_error)?;
    if formatted
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return Ok(None);
    }

    Ok(Some(lsp::TextEdit {
        range: position::range(file, Span::new(file.id, 0, file.len))?,
        new_text: formatted.text,
    }))
}

/// Format one file range and return one LSP edit.
pub(super) fn range_edit(
    file: &File,
    formatter: FormatterOptions,
    start: u32,
    end: u32,
) -> jsonrpc::Result<Option<lsp::TextEdit>> {
    let formatted =
        format_source_range(file, file.text(), formatter, start, end).map_err(internal_error)?;
    if formatted
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return Ok(None);
    }
    let Some(edit) = formatted.edit else {
        return Ok(None);
    };

    Ok(Some(lsp::TextEdit {
        range: position::range(file, edit.span)?,
        new_text: edit.text,
    }))
}
