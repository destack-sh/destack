use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::context::HtmlFormatContext;
use crate::print::Printer;
use crate::{AttributeValue, AttributeValueForm, Content, Element, LocalNodeId, Name, NodeTree};

/// Write one content node.
pub(crate) fn write_content(
    tree: &NodeTree,
    content_id: LocalNodeId<Content>,
    is_raw_text: bool,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    match tree.get(content_id) {
        Content::Element(element) => write_element(tree, element, f),
        Content::Text(text_node) => write_text(&text_node.value, is_raw_text, f),
        Content::Comment(comment) => {
            write!(f, [text("<!--"), text(&comment.value), text("-->")])
        }
        Content::Instruction(instruction) => {
            write!(f, [text("<?"), text(&instruction.target)])?;

            if instruction.contents.is_empty() {
                write!(f, [text("?>")])
            } else {
                write!(f, [space(), text(&instruction.contents), text("?>")])
            }
        }
    }
}

/// Write one element.
pub(crate) fn write_element(
    tree: &NodeTree,
    element: &Element,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    // content
    let content = element
        .content
        .map(|content| tree.get(content).children.clone())
        .unwrap_or_else(|| element.children.clone());
    let is_raw_text = Printer::is_raw_text_element_name(&element.name.local);

    // opening tag
    write!(f, [text("<")])?;
    write_authored_or_resolved_name(
        f.context().element_start_tag_name(element),
        &element.name,
        f,
    )?;

    // attributes
    for attribute_id in &element.attributes {
        let attribute = tree.get(*attribute_id);

        write!(f, [space()])?;
        write_authored_or_resolved_name(f.context().attribute_name(attribute), &attribute.name, f)?;

        if let Some(value) = &attribute.value {
            write_attribute_value(value, f)?;
        }
    }

    // self closing
    if element.is_self_closing {
        let self_closing = f.context().render_self_closing_delimiter(element);

        return write!(f, [text(self_closing)]);
    }

    write!(f, [text(">")])?;

    // void elements
    if Printer::is_void_element_name(&element.name.local) {
        return Ok(());
    }

    // empty elements
    if content.is_empty() {
        if element.has_authored_end_tag {
            write!(f, [text("</")])?;
            write_authored_or_resolved_name(
                f.context().element_end_tag_name(element),
                &element.name,
                f,
            )?;
            write!(f, [text(">")])?;
        }

        return Ok(());
    }

    // inline content
    if can_inline_element(tree, &content, is_raw_text) {
        for child in &content {
            write_content(tree, *child, is_raw_text, f)?;
        }

        if element.has_authored_end_tag {
            write!(f, [text("</")])?;
            write_authored_or_resolved_name(
                f.context().element_end_tag_name(element),
                &element.name,
                f,
            )?;
            write!(f, [text(">")])?;
        }

        return Ok(());
    }

    // block content
    write!(
        f,
        [indent(&format_with(|f| {
            for child in &content {
                write!(f, [hard_line_break()])?;
                write_content(tree, *child, is_raw_text, f)?;
            }

            Ok(())
        }))]
    )?;

    if element.has_authored_end_tag {
        write!(f, [hard_line_break(), text("</")])?;
        write_authored_or_resolved_name(
            f.context().element_end_tag_name(element),
            &element.name,
            f,
        )?;
        write!(f, [text(">")])?;
    }

    Ok(())
}

/// Write one authored name when present, otherwise one resolved name.
fn write_authored_or_resolved_name(
    authored_name: Option<&str>,
    name: &Name,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    // authored name
    if let Some(authored_name) = authored_name {
        return write!(f, [text(authored_name)]);
    }

    // qualified fallback
    if let Some(prefix) = &name.prefix {
        write!(f, [text(prefix), text(":")])?;
    }

    write!(f, [text(&name.local)])
}

/// Return whether one element can stay inline.
fn can_inline_element(
    tree: &NodeTree,
    content: &[LocalNodeId<Content>],
    is_raw_text: bool,
) -> bool {
    // only one child can stay inline
    if content.len() != 1 {
        return false;
    }

    // child content
    match tree.get(content[0]) {
        Content::Text(text) => is_raw_text || !text.value.contains('\n'),
        Content::Comment(comment) => !comment.value.contains('\n'),
        Content::Instruction(instruction) => !instruction.contents.contains('\n'),
        Content::Element(_) => false,
    }
}

/// Escape one HTML text payload.
fn write_text(
    value: &str,
    is_raw_text: bool,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    // raw text
    if is_raw_text {
        return write!(f, [text(value)]);
    }

    // escaping
    for character in value.chars() {
        match character {
            '&' => write!(f, [text("&amp;")])?,
            '<' => write!(f, [text("&lt;")])?,
            '>' => write!(f, [text("&gt;")])?,
            _ => write!(f, [text(&character.to_string())])?,
        }
    }

    Ok(())
}

/// Write one authored attribute value form.
fn write_attribute_value(
    value: &AttributeValue,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    match value.form {
        AttributeValueForm::DoubleQuoted => {
            write!(f, [token("="), text("\"")])?;
            write_double_quoted_html_value(&value.value, f)?;
            write!(f, [text("\"")])
        }
        AttributeValueForm::SingleQuoted => {
            write!(f, [token("="), text("'")])?;
            write_single_quoted_html_value(&value.value, f)?;
            write!(f, [text("'")])
        }
        AttributeValueForm::Unquoted => {
            write!(f, [token("=")])?;
            write_unquoted_html_value(&value.value, f)
        }
    }
}

/// Write one double-quoted HTML value.
pub(crate) fn write_double_quoted_html_value(
    value: &str,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    for character in value.chars() {
        match character {
            '&' => write!(f, [text("&amp;")])?,
            '"' => write!(f, [text("&quot;")])?,
            '<' => write!(f, [text("&lt;")])?,
            '>' => write!(f, [text("&gt;")])?,
            _ => write!(f, [text(&character.to_string())])?,
        }
    }

    Ok(())
}

/// Write one single-quoted HTML value.
pub(crate) fn write_single_quoted_html_value(
    value: &str,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    for character in value.chars() {
        match character {
            '&' => write!(f, [text("&amp;")])?,
            '\'' => write!(f, [text("&#39;")])?,
            '<' => write!(f, [text("&lt;")])?,
            '>' => write!(f, [text("&gt;")])?,
            _ => write!(f, [text(&character.to_string())])?,
        }
    }

    Ok(())
}

/// Write one unquoted HTML value.
pub(crate) fn write_unquoted_html_value(
    value: &str,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    for character in value.chars() {
        match character {
            '&' => write!(f, [text("&amp;")])?,
            '"' => write!(f, [text("&quot;")])?,
            '\'' => write!(f, [text("&#39;")])?,
            '<' => write!(f, [text("&lt;")])?,
            '>' => write!(f, [text("&gt;")])?,
            '=' => write!(f, [text("&#61;")])?,
            '`' => write!(f, [text("&#96;")])?,
            character if character.is_ascii_whitespace() => write!(f, [text("&#32;")])?,
            _ => write!(f, [text(&character.to_string())])?,
        }
    }

    Ok(())
}
