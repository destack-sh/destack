use markdown::mdast::Node;
use markdown::{Constructs, ParseOptions, to_mdast};
use tspp_fir::format::{FormatError, FormatResult};

use crate::TsppFormatOptions;

use super::super::line::LineBuffer;

/// Canonical Markdown formatter for one documentation body.
pub(in crate::documentation) struct MarkdownFormatter<'a> {
    /// The available line width.
    pub(super) width: usize,
    /// The surrounding source formatter options.
    pub(super) options: &'a TsppFormatOptions,
}

impl<'a> MarkdownFormatter<'a> {
    /// Create one Markdown formatter.
    pub(in crate::documentation) fn new(width: usize, options: &'a TsppFormatOptions) -> Self {
        Self { width, options }
    }

    /// Format one complete Markdown body.
    pub(in crate::documentation) fn format(&self, text: &str) -> FormatResult<String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }

        // parse the complete Markdown body
        let parse_options = ParseOptions {
            constructs: Constructs {
                gfm_autolink_literal: false,
                gfm_footnote_definition: true,
                gfm_label_start_footnote: true,
                gfm_strikethrough: true,
                gfm_table: true,
                gfm_task_list_item: true,
                ..Constructs::default()
            },
            ..ParseOptions::default()
        };
        let root = to_mdast(text, &parse_options).map_err(|_| FormatError::SyntaxError {
            message: "documentation Markdown could not be parsed",
        })?;

        // format the parsed body
        let mut lines = LineBuffer::new();
        self.format_children(&root, 0, &mut lines)?;

        Ok(lines.into_string())
    }

    /// Format the children of one Markdown node.
    fn format_children(
        &self,
        node: &Node,
        indent: usize,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        let Some(children) = node.children() else {
            return Ok(());
        };

        self.format_nodes(children, indent, lines)
    }

    /// Format one Markdown node sequence.
    fn format_nodes(
        &self,
        nodes: &[Node],
        indent: usize,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        for (index, child) in nodes.iter().enumerate() {
            // separate block siblings
            if index > 0 && Self::is_block_node(child) && !lines.last_is_empty() {
                lines.push_empty();
            }

            self.format_node(child, indent, lines)?;
        }

        Ok(())
    }

    /// Return whether one Markdown node requires block separation.
    pub(super) fn is_block_node(node: &Node) -> bool {
        matches!(
            node,
            Node::Paragraph(_)
                | Node::Heading(_)
                | Node::List(_)
                | Node::Code(_)
                | Node::Blockquote(_)
                | Node::ThematicBreak(_)
                | Node::Table(_)
                | Node::Definition(_)
                | Node::FootnoteDefinition(_)
                | Node::Html(_)
        )
    }

    /// Format one Markdown node.
    pub(super) fn format_node(
        &self,
        node: &Node,
        indent: usize,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        match node {
            Node::Root(_) => {
                self.format_children(node, indent, lines)?;
            }
            Node::Paragraph(paragraph) => {
                self.format_paragraph(paragraph, indent, lines)?;
            }
            Node::Heading(heading) => {
                let text = self.format_inline(node)?;
                let prefix = "#".repeat(heading.depth as usize);
                if !lines.is_empty() && !lines.last_is_empty() {
                    lines.push_empty();
                }
                let line = lines.begin_line();
                line.push_str(&prefix);
                line.push(' ');
                line.push_str(&text);
            }
            Node::List(list) => {
                self.format_list(list, indent, lines)?;
            }
            Node::ListItem(_) => {
                return Err(FormatError::SyntaxError {
                    message: "Markdown list item has no list owner",
                });
            }
            Node::ThematicBreak(_) => lines.push("---"),
            Node::Code(code) => {
                self.format_code(code, lines)?;
            }
            Node::Blockquote(blockquote) => {
                self.format_blockquote(blockquote, lines)?;
            }
            Node::Table(table) => {
                self.format_table(table, lines)?;
            }
            Node::Definition(definition) => {
                let label = definition
                    .label
                    .as_deref()
                    .unwrap_or(&definition.identifier);
                let line = lines.begin_line();
                line.push('[');
                line.push_str(label);
                line.push_str("]: ");
                line.push_str(&definition.url);
                if let Some(title) = definition.title.as_deref() {
                    line.push_str(" \"");
                    line.push_str(title);
                    line.push('"');
                }
            }
            Node::FootnoteDefinition(definition) => {
                self.format_footnote_definition(definition, lines)?;
            }
            Node::Html(html) => {
                for line in html.value.lines() {
                    lines.push(line);
                }
            }
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unsupported block Markdown node",
                });
            }
        }

        Ok(())
    }

    /// Format one Markdown footnote definition.
    fn format_footnote_definition(
        &self,
        definition: &markdown::mdast::FootnoteDefinition,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        let mut content = LineBuffer::new();
        self.format_nodes(&definition.children, 0, &mut content)?;
        let label = definition
            .label
            .as_deref()
            .unwrap_or(&definition.identifier);

        // prefix the first line and indent continuations
        for (index, content) in content.into_string().split('\n').enumerate() {
            let line = lines.begin_line();
            if index == 0 {
                line.push_str("[^");
                line.push_str(label);
                line.push_str("]: ");
            } else if !content.is_empty() {
                line.push_str("    ");
            }
            line.push_str(content);
        }

        Ok(())
    }

    /// Format one Markdown paragraph.
    fn format_paragraph(
        &self,
        paragraph: &markdown::mdast::Paragraph,
        indent: usize,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        // hard line breaks
        let has_breaks = paragraph
            .children
            .iter()
            .any(|child| matches!(child, Node::Break(_)));

        if has_breaks {
            // split into break segments
            let indentation = Self::indentation_text(indent);
            let mut current_segment = String::new();

            for child in &paragraph.children {
                if matches!(child, Node::Break(_)) {
                    // emit current segment
                    let text = current_segment.trim();
                    if indent > 0 {
                        let line = lines.begin_line();
                        line.push_str(&indentation);
                        line.push_str(text);
                        line.push('\\');
                    } else {
                        let line = lines.begin_line();
                        line.push_str(text);
                        line.push('\\');
                    }
                    current_segment.clear();
                } else {
                    self.write_inline(child, &mut current_segment)?;
                }
            }

            // emit final segment
            if !current_segment.trim().is_empty() {
                let text = current_segment.trim();
                if indent > 0 {
                    let line = lines.begin_line();
                    line.push_str(&indentation);
                    line.push_str(text);
                } else {
                    lines.push(text);
                }
            }
            return Ok(());
        }

        // collect and wrap inline text
        let inline_text = self.format_inline_children(&paragraph.children)?;
        let effective_width = self.width.saturating_sub(indent);
        let indentation = Self::indentation_text(indent);

        let mut paragraph_lines = LineBuffer::new();
        self.wrap_paragraph(&inline_text, effective_width, 0, 0, &mut paragraph_lines);
        let formatted_paragraph = paragraph_lines.into_string();

        for content in formatted_paragraph.split('\n') {
            if indent > 0 {
                if content.is_empty() {
                    lines.push_empty();
                } else {
                    let line = lines.begin_line();
                    line.push_str(&indentation);
                    line.push_str(content);
                }
            } else {
                lines.push(content);
            }
        }

        Ok(())
    }

    /// Format one Markdown block quote.
    fn format_blockquote(
        &self,
        blockquote: &markdown::mdast::Blockquote,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        // format each blockquote section
        for (index, child) in blockquote.children.iter().enumerate() {
            if index > 0 {
                lines.push_empty();
            }
            let mut formatted = LineBuffer::new();
            self.format_node(child, 0, &mut formatted)?;
            for line in formatted.into_string().split('\n') {
                if line.is_empty() {
                    lines.push(">");
                } else {
                    let output = lines.begin_line();
                    output.push_str("> ");
                    output.push_str(line);
                }
            }
        }

        Ok(())
    }
}
