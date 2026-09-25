use tspp_core::StringId;
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::write;

use super::value::format_type_id;

use crate::{Static, StaticField, StaticId, StaticKey, Tree, Type, TypeId, Writer};

/// Format one closed compile-time value.
pub(super) fn format_static<'a>(id: StaticId, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    let value = f.context().tree.static_value(id);

    match value {
        Static::Parameter(index) => super::r#type::format_parameter(*index, f),
        Static::Null => write!(f, [token("null")]),
        Static::Undefined => write!(f, [token("undefined")]),
        Static::Boolean(value) => write!(f, [token(if *value { "true" } else { "false" })]),
        Static::Integer(value) => write!(f, [copied_text(&value.to_string())]),
        Static::Bigint(value) => write!(f, [copied_text(&format!("{value}n"))]),
        Static::Float(bits) => format_float(*bits, f),
        Static::Character(value) => write!(f, [copied_text(&format!("{value:?}"))]),
        Static::String(value) => {
            let value = f.context().strings.get(*value);

            write!(f, [copied_text(&format!("{value:?}"))])
        }
        Static::Regex { content, flags } => format_regex(*content, *flags, f),
        Static::Type(ty) => format_static_type(*ty, f),
        Static::Array(values) => format_array(values, f),
        Static::FixedArray { value, length } => {
            write!(f, [token("[")])?;
            format_static(*value, f)?;
            write!(
                f,
                [
                    token(";"),
                    space(),
                    copied_text(&length.to_string()),
                    token("]")
                ]
            )
        }
        Static::Tuple(values) => format_tuple(values, f),
        Static::Newtype { ty, value } => format_newtype(*ty, *value, f),
        Static::Object(fields) => format_fields(None, fields, f),
        Static::Struct { ty, fields } => format_fields(Some(*ty), fields, f),
    }
}

/// Format one static floating-point value.
fn format_float<'a>(bits: u64, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    let value = f64::from_bits(bits);
    let text = if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value == 0.0 && value.is_sign_negative() {
        "-0.0".to_string()
    } else {
        let mut text = value.to_string();
        if !text.contains(['.', 'e', 'E']) {
            text.push_str(".0");
        }
        text
    };

    write!(f, [copied_text(&text)])
}

/// Format one static regular expression literal.
fn format_regex<'a>(
    content: StringId,
    flags: Option<StringId>,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let content = f.context().strings.get(content);
    write!(f, [token("/"), copied_text(content), token("/")])?;

    if let Some(flags) = flags {
        let flags = f.context().strings.get(flags);
        write!(f, [copied_text(flags)])?;
    }

    Ok(())
}

/// Format one static type value.
fn format_static_type<'a>(ty: TypeId, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    if static_type_needs_keyword(ty, f.context().tree) {
        write!(f, [token("type"), space()])?;
    }

    format_type_id(ty, f)
}

/// Return whether one type needs an explicit type-space marker in static position.
fn static_type_needs_keyword(ty: TypeId, tree: &Tree) -> bool {
    if tree.type_declaration(ty).is_some() {
        return false;
    }

    match tree.type_definition(ty) {
        Type::FixedArray { .. }
        | Type::Tuple { .. }
        | Type::Struct { .. }
        | Type::FunctionSignature { .. }
        | Type::Function { .. }
        | Type::Parameter { .. } => true,
        Type::Application { base, .. } => static_type_needs_keyword(*base, tree),
        _ => false,
    }
}

/// Format one static array.
fn format_array<'a>(values: &[StaticId], f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token("[")])?;
    format_statics(values, f)?;

    write!(f, [token("]")])
}

/// Format one static tuple.
fn format_tuple<'a>(values: &[StaticId], f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    format_statics(values, f)?;

    if values.len() == 1 {
        write!(f, [token(",")])?;
    }

    write!(f, [token(")")])
}

/// Format one comma-separated static value sequence.
fn format_statics<'a>(values: &[StaticId], f: &mut Writer<'a, '_>) -> FormatResult<()> {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        format_static(*value, f)?;
    }

    Ok(())
}

/// Format one nominal newtype value.
fn format_newtype<'a>(ty: TypeId, value: StaticId, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    format_type_id(ty, f)?;
    write!(f, [token("(")])?;

    match f.context().tree.static_value(value) {
        Static::Tuple(values) => format_statics(values, f)?,
        _ => format_static(value, f)?,
    }

    write!(f, [token(")")])
}

/// Format one structural object or nominal struct value.
fn format_fields<'a>(
    ty: Option<TypeId>,
    fields: &[StaticField],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    if let Some(ty) = ty {
        format_type_id(ty, f)?;
        write!(f, [space()])?;
    }

    write!(f, [token("{")])?;
    if !fields.is_empty() {
        write!(f, [space()])?;
    }

    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        format_static_key(&field.key, f)?;
        write!(f, [token(":"), space()])?;
        format_static(field.value, f)?;
    }

    if !fields.is_empty() {
        write!(f, [space()])?;
    }

    write!(f, [token("}")])
}

/// Format one static object key.
fn format_static_key<'a>(key: &StaticKey, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    match key {
        StaticKey::Name(name) => {
            let name = f.context().strings.get(*name);
            if is_identifier(name) {
                write!(f, [copied_text(name)])
            } else {
                write!(f, [copied_text(&format!("{name:?}"))])
            }
        }
        StaticKey::Index(index) => write!(f, [copied_text(&index.to_string())]),
    }
}

/// Return whether one string is an ordinary source identifier.
fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}
