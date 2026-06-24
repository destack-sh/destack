use crate::generate::implementation::{ImplementationOwner, implementation_owners};
use crate::generate::schema::{Field, Item, Payload, Schema, SchemaModule, Shape, Type, Variant};

use super::codec::{
    decode_name, encode_name, from_json_name, payload_property_name, property_key, to_json_name,
};
use super::name::{identifier, member};
use super::path::TypeNames;
use super::text::Text;

/// Render one client item as a TypeScript type.
pub(super) fn render_item(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
    type_names: &TypeNames,
    text: &mut Text,
) {
    let owner = implementation_owners(module)
        .into_iter()
        .find(|owner| owner.matches(item));

    ItemRenderer {
        schema,
        item,
        type_names,
        owner,
    }
    .render(text);
}

/// TypeScript item renderer.
struct ItemRenderer<'schema> {
    /// Bridge schema.
    schema: &'schema Schema,
    /// Bridge item.
    item: &'schema Item,
    /// Visible TypeScript type names.
    type_names: &'schema TypeNames,
    /// Handwritten implementation owner.
    owner: Option<&'static ImplementationOwner>,
}

impl<'schema> ItemRenderer<'schema> {
    /// Render this TypeScript item.
    fn render(&self, text: &mut Text) {
        let item = self.item;
        text.doc(item.doc(), "");

        if let Some(ty) = item.scalar_newtype() {
            text.line(format!(
                "export type {} = {};",
                item.name,
                render_type(self.schema, self.type_names, ty)
            ));
            text.blank();
            render_struct_companion(text, item, self.owner);

            return;
        }

        match &item.shape {
            Shape::Struct(fields) => {
                text.line(format!("export type {} = {{", item.name));
                for field in fields {
                    render_field(self.schema, self.type_names, text, field, "    ");
                }
                text.line("};");
                text.blank();
                render_struct_companion(text, item, self.owner);
            }
            Shape::Enum(variants) if self.schema.is_unit_enum(&item.key) => {
                text.line(format!(
                    "export type {} = {};",
                    item.name,
                    render_unit_enum(variants)
                ));
                text.blank();
                render_unit_enum_companion(text, item);
            }
            Shape::Enum(variants) => {
                text.line(format!("export type {} =", item.name));
                render_payload_enum(self.schema, self.type_names, text, variants);
                text.line(";");
                text.blank();
                render_payload_constructors(
                    self.schema,
                    self.type_names,
                    text,
                    item,
                    variants,
                    self.owner,
                );
            }
        }
    }
}

/// Render a TypeScript companion object for one struct.
fn render_struct_companion(text: &mut Text, item: &Item, owner: Option<&ImplementationOwner>) {
    text.line(format!("export const {} = {{", item.name));
    if let Some(owner) = owner {
        text.line(format!("    ...{},", owner.typescript_impl()));
        text.blank();
    }
    render_companion_codecs(text, item, "    ");
    text.line("};");
    text.blank();
}

/// Render a TypeScript companion object for one unit enum.
fn render_unit_enum_companion(text: &mut Text, item: &Item) {
    text.line(format!("export const {} = {{", item.name));
    render_companion_codecs(text, item, "    ");
    text.line("};");
    text.blank();
}

/// One generated text document.
fn render_field(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    field: &Field,
    indent: &str,
) {
    let name = property_key(&field.label());
    let ty = render_field_type(schema, type_names, &field.ty);
    let optional = matches!(field.ty, Type::Option(_));
    let marker = if optional { "?" } else { "" };

    text.doc(field.doc(), indent);
    text.line(format!("{indent}readonly {name}{marker}: {ty};"));
}

/// Render one unit enum as a string literal union.
fn render_unit_enum(variants: &[Variant]) -> String {
    variants
        .iter()
        .map(|variant| format!("{:?}", variant.label()))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Render one payload enum as a discriminated union.
fn render_payload_enum(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    variants: &[Variant],
) {
    for variant in variants {
        text.doc(variant.doc(), "    ");
        text.line("    | {");
        text.line(format!("          readonly kind: {:?};", variant.label()));

        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(ty) => {
                let name = variant.payload_field_name();
                let ty = render_type(schema, type_names, ty);

                text.line(format!("          readonly {name}: {ty};"));
            }
            Payload::Struct(fields) => {
                for field in fields {
                    render_payload_field(schema, type_names, text, field, "          ");
                }
            }
        }

        text.line("      }");
    }
}

/// Render constructors for one TypeScript payload enum.
fn render_payload_constructors(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    item: &Item,
    variants: &[Variant],
    owner: Option<&ImplementationOwner>,
) {
    text.line(format!("export const {} = {{", item.name));
    if let Some(owner) = owner {
        text.line(format!("    ...{},", owner.typescript_impl()));
        text.blank();
    }

    for variant in variants {
        let method = member(&variant.label());
        let arguments = render_constructor_arguments(schema, type_names, variant);
        let fields = render_constructor_fields(variant);
        let fields = if fields.is_empty() {
            String::new()
        } else {
            format!(", {}", fields.join(", "))
        };

        text.doc(variant.doc(), "    ");
        text.line(format!("    {method}({arguments}): {} {{", item.name));
        text.line(format!(
            "        return {{ kind: {:?}{fields} }};",
            variant.label()
        ));
        text.line("    },");
        text.blank();
    }

    render_companion_codecs(text, item, "    ");
    text.line("};");
    text.blank();
}

/// Render TypeScript companion codec methods.
fn render_companion_codecs(text: &mut Text, item: &Item, indent: &str) {
    let encode = encode_name(&item.name);
    let decode = decode_name(&item.name);
    let to_json = to_json_name(&item.name);
    let from_json = from_json_name(&item.name);

    text.doc("Encode this value.", indent);
    text.line(format!(
        "{indent}encode(writer: BinaryWriter, value: {}): void {{",
        item.name
    ));
    text.line(format!("{indent}    {encode}(writer, value);"));
    text.line(format!("{indent}}},"));
    text.blank();

    text.doc(&format!("Decode one {}.", item.name), indent);
    text.line(format!(
        "{indent}decode(reader: BinaryReader): {} {{",
        item.name
    ));
    text.line(format!("{indent}    return {decode}(reader);"));
    text.line(format!("{indent}}},"));
    text.blank();

    text.doc("Return this value as JSON.", indent);
    text.line(format!("{indent}toJson(value: {}): Json {{", item.name));
    text.line(format!("{indent}    return {to_json}(value);"));
    text.line(format!("{indent}}},"));
    text.blank();

    text.doc(
        &format!("Return one {} from one JSON value.", item.name),
        indent,
    );
    text.line(format!("{indent}fromJson(value: Json): {} {{", item.name));
    text.line(format!("{indent}    return {from_json}(value);"));
    text.line(format!("{indent}}},"));
}

/// Render TypeScript constructor arguments for one payload variant.
fn render_constructor_arguments(
    schema: &Schema,
    type_names: &TypeNames,
    variant: &Variant,
) -> String {
    constructor_fields(variant)
        .into_iter()
        .map(|(name, ty)| {
            let parameter = ts_parameter_name(&name);

            format!("{parameter}: {}", render_type(schema, type_names, &ty))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render TypeScript object fields for one payload constructor.
fn render_constructor_fields(variant: &Variant) -> Vec<String> {
    constructor_fields(variant)
        .into_iter()
        .map(|(name, _ty)| {
            let property = payload_property_name(&name);
            let parameter = ts_parameter_name(&name);
            if parameter == property {
                property
            } else {
                format!("{}: {parameter}", property_key(&property))
            }
        })
        .collect()
}

/// Return TypeScript constructor fields for one payload variant.
fn constructor_fields(variant: &Variant) -> Vec<(String, Type)> {
    match &variant.payload {
        Payload::Unit => Vec::new(),
        Payload::Tuple(ty) => vec![(variant.payload_field_name(), ty.clone())],
        Payload::Struct(fields) => fields
            .iter()
            .map(|field| (field.label(), field.ty.clone()))
            .collect(),
    }
}

/// Return one valid TypeScript parameter name.
fn ts_parameter_name(name: &str) -> String {
    identifier(name)
}

/// Render one payload enum field type.
fn render_payload_field(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    field: &Field,
    indent: &str,
) {
    let name = property_key(&payload_property_name(&field.label()));
    let ty = render_field_type(schema, type_names, &field.ty);
    let optional = matches!(field.ty, Type::Option(_));
    let marker = if optional { "?" } else { "" };

    text.doc(field.doc(), indent);
    text.line(format!("{indent}readonly {name}{marker}: {ty};"));
}

/// Render one TypeScript field type.
fn render_field_type(schema: &Schema, type_names: &TypeNames, ty: &Type) -> String {
    match ty {
        Type::Option(ty) => render_type(schema, type_names, ty),
        _ => render_type(schema, type_names, ty),
    }
}

/// Render one TypeScript type.
pub(super) fn render_type(schema: &Schema, type_names: &TypeNames, ty: &Type) -> String {
    match ty {
        Type::String => "string".to_string(),
        Type::Bool => "boolean".to_string(),
        Type::Char => "string".to_string(),
        Type::U8 | Type::U32 | Type::Usize | Type::Signed(8 | 16 | 32) | Type::Float(_) => {
            "number".to_string()
        }
        Type::U64 | Type::U128 | Type::Signed(64 | 128) => "bigint".to_string(),
        Type::Signed(_) => ty.unsupported_client_type(),
        Type::Vec(ty) if matches!(ty.as_ref(), Type::U8) => {
            "Uint8Array | readonly number[]".to_string()
        }
        Type::Vec(ty) => format!("ReadonlyArray<{}>", render_type(schema, type_names, ty)),
        Type::Option(ty) => format!("{} | undefined", render_type(schema, type_names, ty)),
        Type::Array(ty, len) => render_array_type(schema, type_names, ty, *len),
        Type::Tuple(types) => {
            let types = types
                .iter()
                .map(|ty| render_type(schema, type_names, ty))
                .collect::<Vec<_>>()
                .join(", ");

            format!("readonly [{types}]")
        }
        Type::Map(key, value) if matches!(key.as_ref(), Type::String) => {
            format!(
                "Readonly<Record<string, {}>>",
                render_type(schema, type_names, value)
            )
        }
        Type::Map(key, value) => format!(
            "ReadonlyMap<{}, {}>",
            render_type(schema, type_names, key),
            render_type(schema, type_names, value)
        ),
        Type::Json => "unknown".to_string(),
        Type::Named { key, .. } if schema.is_unit_enum(key) => type_names.ty(ty),
        Type::Named { .. } => type_names.ty(ty),
    }
}

/// Render one fixed-length array type.
fn render_array_type(schema: &Schema, type_names: &TypeNames, ty: &Type, len: usize) -> String {
    if matches!(ty, Type::U8) {
        return "Uint8Array | readonly number[]".to_string();
    }

    let ty = render_type(schema, type_names, ty);
    let fields = std::iter::repeat_n(ty, len).collect::<Vec<_>>().join(", ");

    format!("readonly [{fields}]")
}
