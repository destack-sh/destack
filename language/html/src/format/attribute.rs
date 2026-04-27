use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::context::HtmlFormatContext;
use super::name::write_authored_or_resolved_name;
use crate::{Attribute, AttributeValue, AttributeValueForm, Tree};

/// Write one authored attribute.
pub(crate) fn write_attribute(
    tree: &Tree,
    attribute: &Attribute,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    write_authored_or_resolved_name(tree, attribute.authored_name, &attribute.name, f)?;

    if let Some(value) = &attribute.value {
        write_attribute_value(value, f)?;
    }

    Ok(())
}

/// Write one authored attribute value form.
pub(crate) fn write_attribute_value(
    value: &AttributeValue,
    f: &mut Formatter<'_, HtmlFormatContext>,
) -> FormatResult<()> {
    match value.form {
        AttributeValueForm::DoubleQuoted => {
            write!(f, [token("="), text("\"")])?;
            write_double_quoted_value(&value.value, f)?;
            write!(f, [text("\"")])
        }
        AttributeValueForm::SingleQuoted => {
            write!(f, [token("="), text("'")])?;
            write_single_quoted_value(&value.value, f)?;
            write!(f, [text("'")])
        }
        AttributeValueForm::Unquoted => {
            write!(f, [token("=")])?;
            write_unquoted_value(&value.value, f)
        }
    }
}

/// Write one double-quoted HTML value.
pub(crate) fn write_double_quoted_value(
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
pub(crate) fn write_single_quoted_value(
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
pub(crate) fn write_unquoted_value(
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
