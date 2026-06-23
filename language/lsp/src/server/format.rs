use destack_formatter::{FormatFileError, format_source, format_source_range};
use destack_lsp_types::TextEdit;
use destack_repository::FormatterOptions;
use destack_source::{DiagnosticSeverity, File, Span};

use super::position::byte_span_to_range;

/// Format one file image and return a document edit.
pub(super) fn file_edit(
    file: &File,
    formatter: FormatterOptions,
) -> Result<Option<TextEdit>, FormatFileError> {
    let formatted = format_source(file, file.text(), formatter)?;
    if formatted
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return Ok(None);
    }

    Ok(Some(TextEdit {
        range: byte_span_to_range(file, Span::new(file.id, 0, file.len)),
        new_text: formatted.text,
    }))
}

/// Format one file range and return one LSP edit.
pub(super) fn range_edit(
    file: &File,
    formatter: FormatterOptions,
    start: u32,
    end: u32,
) -> Result<Option<TextEdit>, FormatFileError> {
    let formatted = format_source_range(file, file.text(), formatter, start, end)?;
    if formatted
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return Ok(None);
    }
    let Some(edit) = formatted.edit else {
        return Ok(None);
    };

    Ok(Some(TextEdit {
        range: byte_span_to_range(file, edit.span),
        new_text: edit.text,
    }))
}
