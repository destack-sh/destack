use markdown::mdast::{Node, ReferenceKind};
use tspp_fir::format::{FormatError, FormatResult};

use super::MarkdownFormatter;

impl MarkdownFormatter<'_> {
    /// Format one inline Markdown node.
    pub(super) fn format_inline(&self, node: &Node) -> FormatResult<String> {
        let mut output = String::new();
        self.write_inline(node, &mut output)?;

        Ok(output)
    }

    /// Format one inline Markdown child sequence.
    pub(super) fn format_inline_children(&self, children: &[Node]) -> FormatResult<String> {
        let mut output = String::new();
        for child in children {
            self.write_inline(child, &mut output)?;
        }

        Ok(output)
    }

    /// Write one inline Markdown node.
    pub(super) fn write_inline(&self, node: &Node, output: &mut String) -> FormatResult<()> {
        match node {
            Node::Text(text) => output.push_str(&text.value),
            Node::Emphasis(emphasis) => {
                output.push('_');
                for child in &emphasis.children {
                    self.write_inline(child, output)?;
                }
                output.push('_');
            }
            Node::Strong(strong) => {
                output.push_str("**");
                for child in &strong.children {
                    self.write_inline(child, output)?;
                }
                output.push_str("**");
            }
            Node::InlineCode(code) => {
                output.push('`');
                output.push_str(&code.value);
                output.push('`');
            }
            Node::InlineMath(math) => {
                output.push('$');
                output.push_str(&math.value);
                output.push('$');
            }
            Node::Link(link) => {
                output.push('[');
                for child in &link.children {
                    self.write_inline(child, output)?;
                }
                output.push_str("](");
                output.push_str(&link.url);
                if let Some(title) = &link.title {
                    output.push_str(" \"");
                    output.push_str(title);
                    output.push('"');
                }
                output.push(')');
            }
            Node::LinkReference(reference) => {
                output.push('[');
                for child in &reference.children {
                    self.write_inline(child, output)?;
                }
                output.push(']');
                Self::write_reference(
                    reference.label.as_deref(),
                    &reference.identifier,
                    reference.reference_kind,
                    output,
                );
            }
            Node::Image(image) => {
                output.push_str("![");
                output.push_str(&image.alt);
                output.push_str("](");
                output.push_str(&image.url);
                if let Some(title) = &image.title {
                    output.push_str(" \"");
                    output.push_str(title);
                    output.push('"');
                }
                output.push(')');
            }
            Node::ImageReference(reference) => {
                output.push_str("![");
                output.push_str(&reference.alt);
                output.push(']');
                Self::write_reference(
                    reference.label.as_deref(),
                    &reference.identifier,
                    reference.reference_kind,
                    output,
                );
            }
            Node::Break(_) => output.push_str("\\\n"),
            Node::Delete(delete) => {
                output.push_str("~~");
                for child in &delete.children {
                    self.write_inline(child, output)?;
                }
                output.push_str("~~");
            }
            Node::FootnoteReference(reference) => {
                output.push_str("[^");
                output.push_str(&reference.identifier);
                output.push(']');
            }
            Node::Html(html) => output.push_str(&html.value),
            Node::Paragraph(paragraph) => {
                for child in &paragraph.children {
                    self.write_inline(child, output)?;
                }
            }
            Node::Heading(heading) => {
                for child in &heading.children {
                    self.write_inline(child, output)?;
                }
            }
            Node::TableCell(cell) => {
                for child in &cell.children {
                    self.write_inline(child, output)?;
                }
            }
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unsupported inline Markdown node",
                });
            }
        }

        Ok(())
    }

    /// Write one Markdown reference suffix.
    fn write_reference(
        label: Option<&str>,
        identifier: &str,
        kind: ReferenceKind,
        output: &mut String,
    ) {
        match kind {
            ReferenceKind::Full => {
                output.push('[');
                output.push_str(label.unwrap_or(identifier));
                output.push(']');
            }
            ReferenceKind::Collapsed => output.push_str("[]"),
            ReferenceKind::Shortcut => {}
        }
    }
}
