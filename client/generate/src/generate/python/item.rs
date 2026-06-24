use crate::generate::core::upper_camel;
use crate::generate::implementation::{ImplementationOwner, implementation_owners};
use crate::generate::schema::{Field, Item, Payload, Schema, SchemaModule, Shape, Type, Variant};

use super::codec::{decode_name, encode_name, from_json_name, to_json_name};
use super::name::{python_field_name, python_parameter_name, python_payload_field_name};
use super::path::absolute_module_path;
use super::text::Text;

/// Render one generated Python item.
pub(super) fn render_item(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
    is_stub: bool,
) -> String {
    let owner = implementation_owners(module)
        .into_iter()
        .find(|owner| owner.matches(item));

    if let Some(ty) = item.scalar_newtype() {
        return render_scalar_newtype(schema, item, ty);
    }

    match &item.shape {
        Shape::Struct(fields) => render_struct(schema, item, fields, owner, is_stub),
        Shape::Enum(variants) if schema.is_unit_enum(&item.key) => render_unit_enum(item, variants),
        Shape::Enum(variants) => render_payload_enum(schema, item, variants, owner, is_stub),
    }
}

/// Render one generated Python scalar newtype alias.
fn render_scalar_newtype(schema: &Schema, item: &Item, ty: &Type) -> String {
    let mut text = Text::new();
    let ty = render_type(schema, item, ty);

    text.doc(item.doc(), "");
    text.line(format!("{}: typing.TypeAlias = {ty}", item.name));
    text.blank();

    text.finish()
}

/// Render one generated Python struct.
fn render_struct(
    schema: &Schema,
    item: &Item,
    fields: &[Field],
    owner: Option<&ImplementationOwner>,
    is_stub: bool,
) -> String {
    let mut text = Text::new();
    let base = owner
        .map(|owner| format!("({})", owner.python_impl()))
        .unwrap_or_default();

    text.line("@dataclass(frozen=True, slots=True)");
    text.line(format!("class {}{base}:", item.name));
    text.doc(item.doc(), "    ");

    for field in fields {
        let name = python_field_name(&field.name);
        let ty = render_type(schema, item, &field.ty);

        text.field_doc(field.doc(), "    ");
        text.line(format!("    {name}: {ty}"));
    }
    if !fields.is_empty() {
        text.blank();
    }

    render_struct_methods(&mut text, item, is_stub);
    text.blank();

    text.finish()
}

/// Render one generated Python unit enum.
fn render_unit_enum(item: &Item, variants: &[Variant]) -> String {
    let mut text = Text::new();
    text.doc(item.doc(), "");
    text.line(format!(
        "{}: typing.TypeAlias = {}",
        item.name,
        render_literal_union(variants)
    ));
    text.blank();

    text.finish()
}

/// Render one generated Python payload enum.
fn render_payload_enum(
    schema: &Schema,
    item: &Item,
    variants: &[Variant],
    owner: Option<&ImplementationOwner>,
    is_stub: bool,
) -> String {
    let mut text = Text::new();
    let mut names = Vec::new();

    for variant in variants {
        let name = protocol_variant_name(schema, item, variant);
        names.push(name.clone());
        render_variant(schema, &mut text, item, variant, &name, owner, is_stub);
    }

    text.doc(item.doc(), "");
    text.line(format!(
        "{}: typing.TypeAlias = {}",
        item.name,
        names.join(" | ")
    ));
    text.blank();

    text.finish()
}

/// Render one generated Python payload variant.
fn render_variant(
    schema: &Schema,
    text: &mut Text,
    item: &Item,
    variant: &Variant,
    name: &str,
    owner: Option<&ImplementationOwner>,
    is_stub: bool,
) {
    let base = owner
        .map(|owner| format!("({})", owner.python_impl()))
        .unwrap_or_default();

    text.line("@dataclass(frozen=True, slots=True)");
    text.line(format!("class {name}{base}:"));
    text.doc(variant.doc(), "    ");

    match &variant.payload {
        Payload::Unit => {}
        Payload::Tuple(ty) => {
            let field = python_parameter_name(&variant.payload_field_name());
            let ty = render_type(schema, item, ty);

            text.line(format!("    {field}: {ty}"));
        }
        Payload::Struct(fields) => {
            for field in fields {
                let name = python_payload_field_name(&field.name);
                let ty = render_type(schema, item, &field.ty);

                text.field_doc(field.doc(), "    ");
                text.line(format!("    {name}: {ty}"));
            }
        }
    }

    text.line(format!(
        "    kind: typing.Literal[{:?}] = {:?}",
        variant.label(),
        variant.label()
    ));
    text.blank();
    render_variant_methods(text, item, is_stub);
    text.blank();
}

/// Render Python codec methods for one struct class.
fn render_struct_methods(text: &mut Text, item: &Item, is_stub: bool) {
    let encode = encode_name(&item.name);
    let decode = decode_name(&item.name);
    let to_json = to_json_name(&item.name);
    let from_json = from_json_name(&item.name);

    text.line("    def encode(self, writer: BinaryWriter) -> None:");
    if is_stub {
        text.line("        ...");
    } else {
        text.line("        \"\"\"Encode this value.\"\"\"");
        text.line(format!("        {encode}(writer, self)"));
    }
    text.blank();

    text.line("    @classmethod");
    text.line(format!(
        "    def decode(cls, reader: BinaryReader) -> {}:",
        item.name
    ));
    if is_stub {
        text.line("        ...");
    } else {
        text.line(format!("        \"\"\"Decode one {}.\"\"\"", item.name));
        text.line(format!("        return {decode}(reader)"));
    }
    text.blank();

    text.line("    def to_json(self) -> Json:");
    if is_stub {
        text.line("        ...");
    } else {
        text.line("        \"\"\"Return this value as JSON.\"\"\"");
        text.line(format!("        return {to_json}(self)"));
    }
    text.blank();

    text.line("    @classmethod");
    text.line(format!(
        "    def from_json(cls, value: Json) -> {}:",
        item.name
    ));
    if is_stub {
        text.line("        ...");
    } else {
        text.line(format!(
            "        \"\"\"Return one {} from one JSON value.\"\"\"",
            item.name
        ));
        text.line(format!("        return {from_json}(value)"));
    }
}

/// Render Python codec methods for one payload variant class.
fn render_variant_methods(text: &mut Text, item: &Item, is_stub: bool) {
    let encode = encode_name(&item.name);
    let to_json = to_json_name(&item.name);

    text.line("    def encode(self, writer: BinaryWriter) -> None:");
    if is_stub {
        text.line("        ...");
    } else {
        text.line("        \"\"\"Encode this value.\"\"\"");
        text.line(format!("        {encode}(writer, self)"));
    }
    text.blank();

    text.line("    def to_json(self) -> Json:");
    if is_stub {
        text.line("        ...");
    } else {
        text.line("        \"\"\"Return this value as JSON.\"\"\"");
        text.line(format!("        return {to_json}(self)"));
    }
}

/// Return one generated Python payload variant class name.
pub(super) fn protocol_variant_name(schema: &Schema, item: &Item, variant: &Variant) -> String {
    let name = format!("{}{}", item.name, upper_camel(&variant.label()));
    let path = schema.module_path(&item.key);
    let collides = schema
        .modules
        .iter()
        .find(|module| &module.path == path)
        .is_some_and(|module| module.keys.iter().any(|key| schema.item(key).name == name));

    if collides {
        format!("{name}Variant")
    } else {
        name
    }
}

/// Return one generated Python literal union.
fn render_literal_union(variants: &[Variant]) -> String {
    variants
        .iter()
        .map(|variant| format!("typing.Literal[{:?}]", variant.label()))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Render one generated Python type.
pub(super) fn render_type(schema: &Schema, item: &Item, ty: &Type) -> String {
    match ty {
        Type::String => "str".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Char => "str".to_string(),
        Type::U8 | Type::U32 | Type::U64 | Type::U128 | Type::Signed(_) | Type::Usize => {
            "int".to_string()
        }
        Type::Float(_) => "float".to_string(),
        Type::Vec(ty) if matches!(ty.as_ref(), Type::U8) => {
            "builtins.bytes | bytearray | Sequence[int]".to_string()
        }
        Type::Vec(ty) => format!("Sequence[{}]", render_type(schema, item, ty)),
        Type::Option(ty) => format!("{} | None", render_type(schema, item, ty)),
        Type::Array(ty, len) => render_array_type(schema, item, ty, *len),
        Type::Tuple(types) => render_tuple_type(schema, item, types),
        Type::Map(key, value) => format!(
            "Mapping[{}, {}]",
            render_type(schema, item, key),
            render_type(schema, item, value)
        ),
        Type::Json => "typing.Any".to_string(),
        Type::Named { key, name } if schema.module_path(key) == schema.module_path(&item.key) => {
            name.clone()
        }
        Type::Named { key, name } => {
            let path = absolute_module_path(schema, schema.module_path(key));

            format!("{path}.{name}")
        }
    }
}

/// Render one generated Python fixed array type.
fn render_array_type(schema: &Schema, item: &Item, ty: &Type, len: usize) -> String {
    if matches!(ty, Type::U8) {
        return "builtins.bytes | bytearray | Sequence[int]".to_string();
    }

    let fields = std::iter::repeat_with(|| render_type(schema, item, ty))
        .take(len)
        .collect::<Vec<_>>()
        .join(", ");

    format!("tuple[{fields}]")
}

/// Render one generated Python tuple type.
fn render_tuple_type(schema: &Schema, item: &Item, types: &[Type]) -> String {
    let types = types
        .iter()
        .map(|ty| render_type(schema, item, ty))
        .collect::<Vec<_>>()
        .join(", ");

    format!("tuple[{types}]")
}
