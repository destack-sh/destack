use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

use crate::{
    Attribute, AttributeArgs, AttributeValue, MirFormatter, NodeTree, write_comments_before,
};

/// Write a list of attributes as standalone lines.
pub(crate) fn write_attributes<'a>(
    attributes: &[Attribute],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // render each attribute on its own line
    for attribute in attributes {
        write_attribute(attribute, f)?;
        write!(f, [hard_line_break()])?;
    }

    Ok(())
}

/// Write attributes and their intervening comments before one anchor.
pub(crate) fn write_attributes_before_anchor<'a>(
    attributes: &[Attribute],
    attribute_spans: &[Span],
    anchor_start: u32,
    tree: &NodeTree,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    if attribute_spans.len() != attributes.len() {
        write_attributes(attributes, f)?;
        return Ok(());
    }

    let mut previous_end = None;

    for (index, attribute) in attributes.iter().enumerate() {
        let attribute_span = attribute_spans[index];

        if let Some(previous_end) = previous_end {
            write_comments_before(tree, previous_end, attribute_span.start, f)?;
        }

        write_attribute(attribute, f)?;
        write!(f, [hard_line_break()])?;
        previous_end = Some(attribute_span.end);
    }

    if let Some(previous_end) = previous_end {
        write_comments_before(tree, previous_end, anchor_start, f)?;
    }

    Ok(())
}

/// Write a list of attributes inline.
pub(crate) fn write_inline_attributes<'a>(
    attributes: &[Attribute],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // render attributes inline
    for (index, attribute) in attributes.iter().enumerate() {
        if index > 0 {
            write!(f, [space()])?;
        }

        write_attribute(attribute, f)?;
    }

    Ok(())
}

/// Write a single attribute.
pub(crate) fn write_attribute<'a>(
    attribute: &Attribute,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // open the attribute
    let name = f.context().strings.get(attribute.name);
    write!(f, [token("@"), text(name)])?;

    // format optional arguments
    match &attribute.args {
        AttributeArgs::None => {}
        AttributeArgs::Value(value) => {
            write!(f, [token("(")])?;
            write_attribute_value(value, f)?;
            write!(f, [token(")")])?;
        }
        AttributeArgs::Values(values) => {
            write!(f, [token("(")])?;
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }

                write_attribute_value(value, f)?;
            }
            write!(f, [token(")")])?;
        }
        AttributeArgs::KeyValues(pairs) => {
            write!(f, [token("(")])?;
            for (index, pair) in pairs.iter().enumerate() {
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }

                let key = f.context().strings.get(pair.key);
                write!(f, [text(key), token("=")])?;
                write_attribute_value(&pair.value, f)?;
            }
            write!(f, [token(")")])?;
        }
    }

    Ok(())
}

/// Write a single attribute value.
pub(crate) fn write_attribute_value<'a>(
    value: &AttributeValue,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // format by value kind
    match value {
        AttributeValue::Identifier(name) => {
            let text_value = f.context().strings.get(*name);
            write!(f, [text(text_value)])
        }
        AttributeValue::Type(ty) => write!(f, [*ty]),
        AttributeValue::Integer(value) => write!(f, [text(&value.to_string())]),
        AttributeValue::Float(value) => format_float_literal(*value, f),
        AttributeValue::Boolean(value) => {
            let text_value = if *value { "true" } else { "false" };
            write!(f, [text(text_value)])
        }
        AttributeValue::String(value) => {
            let text_value = f.context().strings.get(*value);
            format_string_literal(text_value, f)
        }
        AttributeValue::List(values) => {
            write!(f, [token("[")])?;
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }

                write_attribute_value(value, f)?;
            }
            write!(f, [token("]")])
        }
    }
}

/// Format a string literal for attributes.
fn format_string_literal<'a>(value: &str, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    // emit escaped string literal contents
    write!(f, [token("\"")])?;
    for ch in value.chars() {
        if ch == '"' {
            write!(f, [text("\\\"")])?;
        } else if ch == '\\' {
            write!(f, [text("\\\\")])?;
        } else if ch == '\n' {
            write!(f, [text("\\n")])?;
        } else if ch == '\r' {
            write!(f, [text("\\r")])?;
        } else if ch == '\t' {
            write!(f, [text("\\t")])?;
        } else if ch.is_ascii_graphic() || ch == ' ' {
            write!(f, [text(&ch.to_string())])?;
        } else {
            write!(f, [text(&format!("\\u{{{:x}}}", ch as u32))])?;
        }
    }
    write!(f, [token("\"")])
}

/// Format a floating point literal for attributes.
fn format_float_literal<'a>(
    value: crate::FloatValue,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // emit a minimal float representation
    let text_value = value.to_f64().to_string();
    write!(f, [text(&text_value)])
}
