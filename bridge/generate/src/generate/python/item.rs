use crate::generate::core::upper_camel;
use crate::generate::schema::{Field, Item, Payload, Schema, Shape, Type, Variant};

use super::name::{python_field_name, python_parameter_name};
use super::text::Text;

/// Render one generated Python protocol item.
pub(super) fn render_protocol_item(schema: &Schema, item: &Item) -> String {
    match &item.shape {
        Shape::Struct(fields) => render_protocol_struct(schema, item, fields),
        Shape::Enum(variants) if schema.is_unit_enum(&item.name) => {
            render_protocol_unit_enum(item, variants)
        }
        Shape::Enum(variants) => render_protocol_payload_enum(schema, item, variants),
    }
}

/// Render one generated Python protocol struct.
fn render_protocol_struct(schema: &Schema, item: &Item, fields: &[Field]) -> String {
    let mut text = Text::new();
    text.line("@dataclass(frozen=True, slots=True)");
    text.line(format!("class {}:", item.name));
    text.doc(item.doc(), "    ");

    if fields.is_empty() {
        text.line("    pass");
    } else {
        for field in fields {
            let name = python_field_name(&field.name);
            let ty = render_protocol_type(schema, &field.ty);

            text.doc(field.doc(), "    ");
            text.line(format!("    {name}: {ty}"));
        }
    }

    text.blank();

    text.finish()
}

/// Render one generated Python protocol unit enum.
fn render_protocol_unit_enum(item: &Item, variants: &[Variant]) -> String {
    let mut text = Text::new();
    text.doc(item.doc(), "");
    text.line(format!(
        "{}: TypeAlias = {}",
        item.name,
        render_literal_union(variants)
    ));
    text.blank();

    text.finish()
}

/// Render one generated Python protocol payload enum.
fn render_protocol_payload_enum(schema: &Schema, item: &Item, variants: &[Variant]) -> String {
    let mut text = Text::new();
    let mut names = Vec::new();

    for variant in variants {
        let name = protocol_variant_name(item, variant);
        names.push(name.clone());
        render_protocol_variant(schema, &mut text, variant, &name);
    }

    text.doc(item.doc(), "");
    text.line(format!("{}: TypeAlias = {}", item.name, names.join(" | ")));
    text.blank();

    text.finish()
}

/// Render one generated Python protocol payload variant.
fn render_protocol_variant(schema: &Schema, text: &mut Text, variant: &Variant, name: &str) {
    text.line("@dataclass(frozen=True, slots=True)");
    text.line(format!("class {name}:"));
    text.doc(variant.doc(), "    ");

    match &variant.payload {
        Payload::Unit => {}
        Payload::Tuple(ty) => {
            let field = python_parameter_name(&variant.payload_field_name());
            let ty = render_protocol_type(schema, ty);

            text.line(format!("    {field}: {ty}"));
        }
        Payload::Struct(fields) => {
            for field in fields {
                let name = python_field_name(&field.name);
                let ty = render_protocol_type(schema, &field.ty);

                text.doc(field.doc(), "    ");
                text.line(format!("    {name}: {ty}"));
            }
        }
    }

    text.line(format!(
        "    kind: Literal[{:?}] = {:?}",
        variant.label(),
        variant.label()
    ));
    text.blank();
}

/// Return one generated Python payload variant class name.
pub(super) fn protocol_variant_name(item: &Item, variant: &Variant) -> String {
    format!("{}{}", item.name, upper_camel(&variant.label()))
}

/// Return one generated Python literal union.
fn render_literal_union(variants: &[Variant]) -> String {
    variants
        .iter()
        .map(|variant| format!("Literal[{:?}]", variant.label()))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Render one generated Python protocol type.
pub(super) fn render_protocol_type(schema: &Schema, ty: &Type) -> String {
    match ty {
        Type::String => "str".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Char => "str".to_string(),
        Type::U8 | Type::U32 | Type::U64 | Type::U128 | Type::Signed(_) | Type::Usize => {
            "int".to_string()
        }
        Type::Float(_) => "float".to_string(),
        Type::Vec(ty) if matches!(ty.as_ref(), Type::U8) => {
            "bytes | bytearray | Sequence[int]".to_string()
        }
        Type::Vec(ty) => format!("Sequence[{}]", render_protocol_type(schema, ty)),
        Type::Option(ty) => format!("{} | None", render_protocol_type(schema, ty)),
        Type::Array(ty, len) => render_protocol_array_type(schema, ty, *len),
        Type::Tuple(types) => render_protocol_tuple_type(schema, types),
        Type::Map(key, value) => format!(
            "Mapping[{}, {}]",
            render_protocol_type(schema, key),
            render_protocol_type(schema, value)
        ),
        Type::Json => "Any".to_string(),
        Type::Named(name) if schema.is_unit_enum(name) => name.clone(),
        Type::Named(name) => name.clone(),
    }
}

/// Render one generated Python fixed array type.
fn render_protocol_array_type(schema: &Schema, ty: &Type, len: usize) -> String {
    if matches!(ty, Type::U8) {
        return "bytes | bytearray | Sequence[int]".to_string();
    }

    let fields = std::iter::repeat_with(|| render_protocol_type(schema, ty))
        .take(len)
        .collect::<Vec<_>>()
        .join(", ");

    format!("tuple[{fields}]")
}

/// Render one generated Python tuple type.
fn render_protocol_tuple_type(schema: &Schema, types: &[Type]) -> String {
    let types = types
        .iter()
        .map(|ty| render_protocol_type(schema, ty))
        .collect::<Vec<_>>()
        .join(", ");

    format!("tuple[{types}]")
}
