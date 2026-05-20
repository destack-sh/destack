use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::write_attribute;
use super::context::HtmlFormatContext;
use super::name::write_authored_or_resolved_name;
use crate::print::Printer;
use crate::{Content, Element, LocalNodeId, Tree};

/// Write one content node.
pub(crate) fn write_content(
    tree: &Tree,
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
            let target = tree.string(instruction.target);

            write!(f, [text("<?"), text(target)])?;

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
    tree: &Tree,
    element: &Element,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    // synthetic element
    if element.authored_start_tag_name.is_none() {
        let content = element
            .content
            .map(|content| tree.get(content).children.clone())
            .unwrap_or_else(|| element.children.clone());
        let element_name = tree.string(element.name.local);
        let is_raw_text = Printer::is_raw_text_element_name(element_name);

        for child in &content {
            write_content(tree, *child, is_raw_text, f)?;
        }

        return Ok(());
    }

    // content
    let content = element
        .content
        .map(|content| tree.get(content).children.clone())
        .unwrap_or_else(|| element.children.clone());
    let element_name = tree.string(element.name.local);
    let is_raw_text = Printer::is_raw_text_element_name(element_name);

    // opening tag
    write!(f, [text("<")])?;
    write_authored_or_resolved_name(
        tree,
        f.context().element_start_tag_name(element),
        &element.name,
        f,
    )?;

    // attributes
    for attribute_id in &element.attributes {
        let attribute = tree.get(*attribute_id);

        write!(f, [space()])?;
        write_attribute(tree, attribute, f)?;
    }

    // self closing
    if element.is_self_closing {
        let self_closing = f.context().render_self_closing_delimiter(element);

        return write!(f, [text(self_closing)]);
    }

    write!(f, [text(">")])?;

    // void elements
    if Printer::is_void_element_name(element_name) {
        return Ok(());
    }

    // empty elements
    if content.is_empty() {
        if element.has_authored_end_tag {
            write!(f, [text("</")])?;
            write_authored_or_resolved_name(
                tree,
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
                tree,
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
            tree,
            f.context().element_end_tag_name(element),
            &element.name,
            f,
        )?;
        write!(f, [text(">")])?;
    }

    Ok(())
}

/// Return whether one element can stay inline.
fn can_inline_element(tree: &Tree, content: &[LocalNodeId<Content>], is_raw_text: bool) -> bool {
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
