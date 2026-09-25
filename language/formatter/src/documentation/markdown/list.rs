use markdown::mdast::{List, Node};
use tspp_fir::format::{FormatError, FormatResult};

use super::super::line::LineBuffer;
use super::MarkdownFormatter;

impl MarkdownFormatter<'_> {
    /// Format one Markdown list.
    pub(super) fn format_list(
        &self,
        list: &List,
        indent: usize,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        let mut ordinal = list.start.unwrap_or(1);

        for child in &list.children {
            let Node::ListItem(item) = child else {
                return Err(FormatError::SyntaxError {
                    message: "Markdown list contains a non-item child",
                });
            };

            // build the list and task marker
            let mut marker = if list.ordered {
                let mut marker = ordinal.to_string();
                marker.push_str(". ");
                ordinal += 1;

                marker
            } else {
                String::from("- ")
            };
            if let Some(is_checked) = item.checked {
                marker.push_str(if is_checked { "[x] " } else { "[ ] " });
            }
            let marker_width = marker.len();

            // format the item children
            for (index, child) in item.children.iter().enumerate() {
                if index == 0 {
                    self.format_first_list_child(child, &marker, indent, lines)?;
                } else {
                    self.format_list_child(child, marker_width, indent, lines)?;
                }
            }
        }

        Ok(())
    }

    /// Format the first child of one list item.
    fn format_first_list_child(
        &self,
        child: &Node,
        marker: &str,
        indent: usize,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        let formatted = self.format_list_child_text(child, marker.len(), true)?;
        let indentation = Self::indentation_text(indent);
        for (index, content) in formatted.split('\n').enumerate() {
            if index == 0 {
                let line = lines.begin_line();
                line.push_str(&indentation);
                line.push_str(marker);
                line.push_str(content);
            } else if content.is_empty() {
                lines.push_empty();
            } else {
                let line = lines.begin_line();
                line.push_str(&indentation);
                line.push_str(content);
            }
        }

        Ok(())
    }

    /// Format one subsequent child of a list item.
    fn format_list_child(
        &self,
        child: &Node,
        marker_width: usize,
        indent: usize,
        lines: &mut LineBuffer,
    ) -> FormatResult<()> {
        if Self::is_block_node(child) && !lines.last_is_empty() {
            lines.push_empty();
        }

        // retain definitions outside the list indentation
        if matches!(child, Node::Definition(_)) {
            self.format_node(child, 0, lines)?;
        }
        // align nested lists beneath the item content
        else if matches!(child, Node::List(_)) {
            self.format_node(child, indent + marker_width, lines)?;
        }
        // align other blocks beneath the item content
        else {
            let formatted = self.format_list_child_text(child, marker_width, false)?;
            let indentation = Self::indentation_text(indent + marker_width);
            for content in formatted.split('\n') {
                if content.is_empty() {
                    lines.push_empty();
                } else {
                    let line = lines.begin_line();
                    line.push_str(&indentation);
                    line.push_str(content);
                }
            }
        }

        Ok(())
    }

    /// Format one list item child into an isolated string.
    fn format_list_child_text(
        &self,
        node: &Node,
        marker_width: usize,
        is_first: bool,
    ) -> FormatResult<String> {
        if let Node::Paragraph(paragraph) = node {
            let text = self.format_inline_children(&paragraph.children)?;
            let continuation_indent = if is_first { marker_width } else { 0 };
            let mut lines = LineBuffer::new();
            self.wrap_paragraph(&text, self.width, 0, continuation_indent, &mut lines);

            Ok(lines.into_string())
        } else {
            let mut lines = LineBuffer::new();
            self.format_node(node, 0, &mut lines)?;

            Ok(lines.into_string())
        }
    }
}
