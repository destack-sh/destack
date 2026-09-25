use std::borrow::Cow;

use markdown::mdast::Code;
use tspp_fir::format::FormatResult;
use tspp_source::FileType;

use super::super::embedded::{fenced_code_file_type, format_embedded_code};
use super::super::line::LineBuffer;
use super::MarkdownFormatter;

impl MarkdownFormatter<'_> {
    /// Format one fenced Markdown code block.
    pub(super) fn format_code(&self, code: &Code, lines: &mut LineBuffer) -> FormatResult<()> {
        if !lines.is_empty() && !lines.last_is_empty() {
            lines.push_empty();
        }

        // format known TS++ code fences
        let width = self.width.saturating_sub(4);
        let formatted = self.format_code_value(&code.value, code.lang.as_deref(), width)?;

        // write the canonical fenced block
        let line = lines.begin_line();
        line.push_str("```");
        if let Some(language) = code.lang.as_deref() {
            line.push_str(language);
        }
        if let Some(metadata) = code.meta.as_deref() {
            line.push(' ');
            line.push_str(metadata);
        }
        for line in formatted.lines() {
            lines.push(line);
        }
        lines.push("```");

        Ok(())
    }

    /// Format one documentation code block.
    fn format_code_value<'code>(
        &self,
        code: &'code str,
        language: Option<&str>,
        width: usize,
    ) -> FormatResult<Cow<'code, str>> {
        // format a declared TS++ language
        let file_type = if let Some(language) = language {
            let Some(file_type) = fenced_code_file_type(language) else {
                return Ok(Cow::Borrowed(code));
            };

            file_type
        } else {
            // use the surrounding source language when no language was declared
            FileType::from(self.options.language_type)
        };

        // keep unformattable snippets verbatim
        match format_embedded_code(code, width, self.options, file_type) {
            Ok(formatted) => Ok(Cow::Owned(formatted)),
            Err(_) => Ok(Cow::Borrowed(code)),
        }
    }
}
