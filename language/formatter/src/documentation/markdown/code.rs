use std::borrow::Cow;

use destack_fir::format::FormatResult;
use destack_source::FileType;
use markdown::mdast::Code;

use super::super::embedded::{fenced_code_file_type, format_embedded_code};
use super::super::line::LineBuffer;
use super::MarkdownFormatter;

impl MarkdownFormatter<'_> {
    /// Format one fenced Markdown code block.
    pub(super) fn format_code(&self, code: &Code, lines: &mut LineBuffer) -> FormatResult<()> {
        if !lines.is_empty() && !lines.last_is_empty() {
            lines.push_empty();
        }

        // format known Destack code fences
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
        // format a declared Destack language
        if let Some(language) = language {
            let Some(file_type) = fenced_code_file_type(language) else {
                return Ok(Cow::Borrowed(code));
            };
            let formatted = format_embedded_code(code, width, self.options, file_type)?;

            return Ok(Cow::Owned(formatted));
        }

        // use the surrounding source language when no language was declared
        let file_type = FileType::from(self.options.language_type);
        let formatted = format_embedded_code(code, width, self.options, file_type)?;

        Ok(Cow::Owned(formatted))
    }
}
