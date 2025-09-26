use std::borrow::Cow;

use crate::{
    ArrayLiteral, DystFormatter, FieldLiteral, FormatNode, NodeId, RangeLiteral, ScalarLiteral,
    StructLiteral, TupleLiteral,
};
use dyst_fir::format::{Format, FormatResult, group, text, token};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, ScalarLiteral> for ScalarLiteral {
    fn format_node(
        &self,
        node_id: NodeId<ScalarLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        let span = f.context().tree.get_span(node_id);
        let span_str = f.context().source.get_span_str(span);
        match self {
            ScalarLiteral::Void => token("void").format(f)?,
            ScalarLiteral::Null => token("null").format(f)?,
            ScalarLiteral::Boolean(value) => {
                token(if *value { "true" } else { "false" }).format(f)?
            }
            ScalarLiteral::Integer(_, _) => {
                let normalized = normalize_integer(span_str);
                text(&normalized).format(f)?;
            }
            ScalarLiteral::Float(_, _) => {
                let normalized = normalize_floating_number(span_str);
                text(&normalized).format(f)?;
            }
            ScalarLiteral::Character(value) => {
                write!(
                    f,
                    [token("'"), text(value.to_string().as_str()), token("'")]
                )?;
            }
            ScalarLiteral::Byte(value) => {
                write!(f, [token("b'"), text(&value.to_string()), token("'")])?;
            }
            ScalarLiteral::String(_) => {
                write!(f, [text(span_str)])?;
            }
            ScalarLiteral::ByteString(_) => {
                write!(f, [text(span_str)])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, RangeLiteral> for RangeLiteral {
    fn format_node(
        &self,
        node_id: NodeId<RangeLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        write!(f, [self.start, token(".."), self.end,])?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, TupleLiteral> for TupleLiteral {
    fn format_node(
        &self,
        node_id: NodeId<TupleLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        write!(
            f,
            [group(&format_args![
                token("("),
                soft_block_indent(&format_with(|f| f
                    .join_with(&format_args![
                        if_group_fits_on_line(&token(",")),
                        soft_line_break_or_space()
                    ])
                    .entries(&self.elements)
                    .finish())),
                f.context().block_infix_annotations(node_id),
                token(")"),
            ])]
        )?;

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, ArrayLiteral> for ArrayLiteral {
    fn format_node(
        &self,
        node_id: NodeId<ArrayLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            ArrayLiteral::Fixed { elements } => {
                write!(
                    f,
                    [group(&format_args![
                        token("["),
                        soft_block_indent(&format_with(|f| f
                            .join_with(&format_args![
                                if_group_fits_on_line(&token(",")),
                                soft_line_break_or_space()
                            ])
                            .entries(elements)
                            .finish())),
                        f.context().block_infix_annotations(node_id),
                        token("]"),
                    ])]
                )?;
            }
        }

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, StructLiteral> for StructLiteral {
    fn format_node(
        &self,
        node_id: NodeId<StructLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        write!(
            f,
            [group(&format_args![
                self.r#type,
                space(),
                token("{"),
                if_group_fits_on_line(&space()),
                soft_block_indent(&format_with(|f| f
                    .join_with(&format_args![
                        if_group_fits_on_line(&token(",")),
                        soft_line_break_or_space()
                    ])
                    .entries(&self.fields)
                    .finish())),
                if_group_fits_on_line(&space()),
                f.context().block_infix_annotations(node_id),
                token("}"),
            ])]
        )?;

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, FieldLiteral> for FieldLiteral {
    fn format_node(
        &self,
        node_id: NodeId<FieldLiteral>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            FieldLiteral::Named { name, value } => {
                write!(f, [name, token(": "), value])?;
            }
            FieldLiteral::NamedShorthand { name } => {
                write!(f, [name])?;
            }
        }

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}

/// Normalize an integer string to canonical form.
///
/// Lowercases prefixes (0b, 0o, 0x) and uppercases hex digits.
fn normalize_integer(input: &str) -> Cow<'_, str> {
    // normalized string if input is not yet normalized
    // output must remain empty if input is already normalized
    let mut output = String::new();
    // tracks the last index of input that has been written to output
    // if last_index is 0 at the end, then the input is already normalized and can be returned as is
    let mut last_index = 0;
    let mut is_hex = false;
    let mut chars = input.char_indices();

    // check if the input starts with a 0 and is followed by a B, O, or X
    if let Some((_, '0')) = chars.next()
        && let Some((index, c)) = chars.next()
    {
        is_hex = matches!(c, 'x' | 'X');
        if matches!(c, 'B' | 'O' | 'X' | 'b' | 'o' | 'x') {
            output.push('0');
            output.push(c.to_ascii_lowercase());
            last_index = index + c.len_utf8();
        }
    }

    // skip the rest if input is not a hex integer because there are only digits
    if is_hex {
        for (index, c) in chars {
            // uppercase hex digits
            if matches!(c, 'a'..='f') {
                output.push_str(&input[last_index..index]);
                output.push(c.to_ascii_uppercase());
                last_index = index + c.len_utf8();
            }
        }
    }

    if last_index == 0 {
        Cow::Borrowed(input)
    } else {
        output.push_str(&input[last_index..]);
        Cow::Owned(output)
    }
}

/// Normalize a floating point number string to canonical form.
///
/// Adds leading/trailing zeros where needed, lowercases exponent, removes plus sign.
fn normalize_floating_number(input: &str) -> Cow<'_, str> {
    // normalized string if input is not yet normalized
    // output must remain empty if input is already normalized
    let mut output = String::new();
    // tracks the last index of input that has been written to output
    // if last_index is 0 at the end, then the input is already normalized and can be returned as is
    let mut last_index = 0;
    let mut chars = input.char_indices();
    let mut prev_char_is_dot = if let Some((index, '.')) = chars.next() {
        // add a leading 0 if input starts with .
        output.push('0');
        output.push('.');
        last_index = index + '.'.len_utf8();
        true
    } else {
        false
    };

    loop {
        match chars.next() {
            Some((index, c @ ('e' | 'E'))) => {
                // add 0 if the e immediately follows a . (e.g., 1.e1)
                if prev_char_is_dot {
                    output.push_str(&input[last_index..index]);
                    output.push('0');
                    last_index = index;
                }

                // lowercase exponent part
                if c == 'E' {
                    output.push_str(&input[last_index..index]);
                    output.push('e');
                    last_index = index + 'E'.len_utf8();
                }

                // remove + in exponent part
                if let Some((index, '+')) = chars.next() {
                    output.push_str(&input[last_index..index]);
                    last_index = index + '+'.len_utf8();
                }

                break;
            }
            Some((_index, c)) => {
                prev_char_is_dot = c == '.';
                continue;
            }
            None => {
                if prev_char_is_dot {
                    // add 0 if fraction part ends with .
                    output.push_str(&input[last_index..]);
                    output.push('0');
                    last_index = input.len();
                }

                break;
            }
        }
    }

    if last_index == 0 {
        Cow::Borrowed(input)
    } else {
        output.push_str(&input[last_index..]);
        Cow::Owned(output)
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_void_literal() {
        assert_format!("void", "void", |p| p.eat_scalar_literal());
    }

    #[test]
    fn test_format_null_literal() {
        assert_format!("null", "null", |p| p.eat_scalar_literal());
    }

    #[test]
    fn test_format_boolean_literal() {
        assert_format!("true", "true", |p| p.eat_scalar_literal());
        assert_format!("false", "false", |p| p.eat_scalar_literal());
    }

    #[test]
    fn test_format_integer_literal() {
        assert_format!(
            "1",
            "1",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_integer_literal_long() {
        assert_format!(
            "1_000_000",
            "1_000_000",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_integer_literal_hex() {
        assert_format!(
            "0x1234",
            "0x1234",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_integer_literal_hex_long() {
        assert_format!(
            "0x1234_5678",
            "0x1234_5678",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_integer_literal_binary() {
        assert_format!(
            "0b1010",
            "0b1010",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_float_literal() {
        assert_format!(
            "1.0",
            "1.0",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_float_literal_long() {
        assert_format!(
            "1.0e38",
            "1.0e38",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_string_literal() {
        assert_format!(
            "\"Hello, world!\"",
            "\"Hello, world!\"",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_string_literal_multiline() {
        assert_format!(
            "\"Hello, world!\nHello, world!\"",
            "\"Hello, world!\nHello, world!\"",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    /// If the tuple fits on a single line, it should print on a single line.
    #[test]
    fn test_format_tuple_literal_short() {
        assert_format!("(1, 2, 3)", "(1, 2, 3)", |p| p.eat_tuple_literal());
    }

    /// If the tuple doesn't fit on a single line, it should print one element per line (indented).
    #[test]
    fn test_format_tuple_literal_long() {
        assert_format!(
            "(1, 2, 3, 4, 5)",
            "(\n\t1\n\t2\n\t3\n\t4\n\t5\n)",
            |p| p.eat_tuple_literal(),
            DystFormatOptions::default_tab_with_line_width(10)
        );
    }

    /// If the array fits on a single line, it should print on a single line.
    #[test]
    fn test_format_array_literal_short() {
        assert_format!("[1, 2, 3]", "[1, 2, 3]", |p| p.eat_array_literal());
    }

    /// If the array doesn't fit on a single line, it should print one element per line (indented).
    #[test]
    fn test_format_array_literal_long() {
        assert_format!(
            "[1, 2, 3, 4, 5]",
            "[\n\t1\n\t2\n\t3\n\t4\n\t5\n]",
            |p| p.eat_array_literal(),
            DystFormatOptions::default_tab_with_line_width(10)
        );
    }

    /// If the struct fits on a single line, it should print on a single line.
    #[test]
    fn test_format_struct_literal_short() {
        assert_format!("Vector2 { x: 1, y: 2 }", "Vector2 { x: 1, y: 2 }", |p| p
            .eat_struct_literal());
    }

    /// If the struct doesn't fit on a single line, it should print one field per line (indented).
    #[test]
    fn test_format_struct_literal_long() {
        assert_format!(
            "Vector4 { x: 1, y: 2, z: 3, w: 4 }",
            "Vector4 {\n\tx: 1\n\ty: 2\n\tz: 3\n\tw: 4\n}",
            |p| p.eat_struct_literal(),
            DystFormatOptions::default_tab_with_line_width(10)
        );
    }

    #[test]
    fn test_format_struct_literal_with_annotations() {
        let source = r"Vector2 { #x x: 0.0, #y y: 0.0 }";
        assert_format!(
            source,
            source,
            |p| p.eat_struct_literal(),
            DystFormatOptions::default()
        );
    }
}
