use tspp_core::{StringId, StringPool};
use tspp_dir::{Documentation, DocumentationTag, StaticKey, Tree};
use tspp_fir::format::{Format, FormatError, FormatResult, Formatter};
use tspp_fir::prelude::{copied_text, hard_line_break, space, token};
use tspp_fir::write;

use crate::{TsppFormatContext, TsppFormatOptions};

use super::line::LineBuffer;
use super::markdown::MarkdownFormatter;

/// Canonically rendered documentation lines.
pub(super) struct FormattedDocumentation {
    /// The rendered lines without comment markers.
    lines: String,
}

/// One documentation rendering operation.
struct DocumentationRenderer<'a> {
    /// The available documentation width.
    width: usize,
    /// The parsed source tree.
    tree: &'a Tree,
    /// The shared source strings.
    strings: &'a StringPool,
    /// The surrounding formatter options.
    options: &'a TsppFormatOptions,
    /// The rendered documentation lines.
    lines: LineBuffer,
}

impl FormattedDocumentation {
    /// Render one parsed documentation group.
    pub(super) fn new(
        documentation: &Documentation,
        width: usize,
        tree: &Tree,
        strings: &StringPool,
        options: &TsppFormatOptions,
    ) -> FormatResult<Self> {
        DocumentationRenderer::new(width, tree, strings, options).render(documentation)
    }
}

impl<'a> Format<'a, TsppFormatContext<'a>> for FormattedDocumentation {
    fn format(&self, formatter: &mut Formatter<'_, 'a, TsppFormatContext<'a>>) -> FormatResult<()> {
        for (index, line) in self.lines.split('\n').enumerate() {
            if index > 0 {
                write!(formatter, [hard_line_break()])?;
            }

            write!(formatter, [token("///")])?;
            if !line.is_empty() {
                write!(formatter, [space(), copied_text(line)])?;
            }
        }

        Ok(())
    }
}

impl<'a> DocumentationRenderer<'a> {
    /// Create one empty documentation renderer.
    fn new(
        width: usize,
        tree: &'a Tree,
        strings: &'a StringPool,
        options: &'a TsppFormatOptions,
    ) -> Self {
        Self {
            width,
            tree,
            strings,
            options,
            lines: LineBuffer::new(),
        }
    }

    /// Render one complete documentation group.
    fn render(mut self, documentation: &Documentation) -> FormatResult<FormattedDocumentation> {
        let markdown = self.strings.get(documentation.markdown);
        let markdown = MarkdownFormatter::new(self.width, self.options).format(markdown)?;
        if !markdown.is_empty() {
            self.lines.push(markdown);
        }

        // render structured entries in authored order
        let mut previous = None;
        for tag in &documentation.tags {
            let follows_prose = previous.is_none();
            let follows_block =
                previous.is_some_and(|tag: DocumentationTag| tag.is_example() || tag.is_section());
            let starts_block = tag.is_example() || tag.is_section();
            let needs_blank =
                !self.lines.is_empty() && (follows_prose || follows_block || starts_block);
            if needs_blank && !self.lines.last_is_empty() {
                self.lines.push_empty();
            }

            self.render_tag(tag)?;
            previous = Some(*tag);
        }

        Ok(FormattedDocumentation {
            lines: self.lines.into_string(),
        })
    }

    /// Render one structured documentation entry.
    fn render_tag(&mut self, tag: &DocumentationTag) -> FormatResult<()> {
        match tag {
            DocumentationTag::Parameter {
                parameter,
                markdown,
            } => {
                self.render_named_tag("@param", self.tree.get(*parameter).symbol_key(), *markdown)?
            }
            DocumentationTag::TypeParameter {
                parameter,
                markdown,
            } => self.render_named_tag(
                "@typeParam",
                self.tree.get(*parameter).symbol_key(),
                *markdown,
            )?,
            DocumentationTag::Example { markdown } => {
                self.lines.push("@example");
                let markdown = self.strings.get(*markdown);
                let markdown = MarkdownFormatter::new(self.width, self.options).format(markdown)?;
                if !markdown.is_empty() {
                    self.lines.push(markdown);
                }
            }
            DocumentationTag::Section { markdown } => {
                // keep authored section lines verbatim
                for line in self.strings.get(*markdown).split('\n') {
                    self.lines.push(line);
                }
            }
        }

        Ok(())
    }

    /// Render one named documentation entry.
    fn render_named_tag(
        &mut self,
        keyword: &str,
        key: Option<StaticKey>,
        markdown: StringId,
    ) -> FormatResult<()> {
        let Some(StaticKey::Name(name)) = key else {
            return Err(FormatError::SyntaxError {
                message: "documentation entry target requires a name",
            });
        };
        let name = self.strings.get(name);
        let prefix = format!("{keyword} {name} - ");
        let markdown = self.strings.get(markdown);
        if markdown.is_empty() {
            return Err(FormatError::SyntaxError {
                message: "documentation entry requires Markdown",
            });
        }
        let width = self.width.saturating_sub(prefix.len());
        let markdown = MarkdownFormatter::new(width, self.options).format(markdown)?;
        let mut markdown = markdown.split('\n');
        let Some(first) = markdown.next() else {
            return Err(FormatError::SyntaxError {
                message: "documentation entry requires Markdown",
            });
        };
        self.lines.push(format!("{prefix}{first}"));

        // align continuation text beneath the description
        let indentation = " ".repeat(prefix.len());
        for content in markdown {
            let line = format!("{indentation}{content}");
            self.lines.push(line.trim_end());
        }

        Ok(())
    }
}
