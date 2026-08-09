use std::iter::repeat_n;

use crate::generate::schema::{Field, Item, Payload, Schema, Type, Variant};

use super::codec::{
    decode_name, encode_name, from_json_name, payload_property_name, property_key, to_json_name,
};
use super::name::{identifier, member};
use super::scope::Scope;
use super::text::Text;

/// Render one client item as a TypeScript type.
pub(super) fn render_item(schema: &Schema, item: &Item, scope: &Scope, text: &mut Text) {
    text.doc(item.doc(), "");

    match &item.ty {
        Type::Struct(fields) => {
            text.line(format!("export type {} = {{", item.name));
            for field in fields {
                render_field(schema, scope, text, field, "    ");
            }
            text.line("};");
            text.blank();
            render_companion(text, item);
        }
        Type::Enum(variants) if schema.is_unit_enum(&item.key) => {
            let variants = render_unit_enum(variants);
            text.line(format!("export type {} = {variants};", item.name));
            text.blank();
            render_companion(text, item);
        }
        Type::Enum(variants) => {
            text.line(format!("export type {} =", item.name));
            render_payload_enum(schema, scope, text, variants);
            text.line(";");
            text.blank();
            render_payload_constructors(schema, scope, text, item, variants);
        }
        ty => {
            let ty = render_type(schema, scope, ty);
            text.line(format!("export type {} = {ty};", item.name));
            text.blank();
            render_companion(text, item);
        }
    }
}

/// Render a TypeScript companion object for one item.
fn render_companion(text: &mut Text, item: &Item) {
    text.line(format!("export const {} = {{", item.name));
    render_companion_codecs(text, item, "    ");
    text.line("};");
    text.blank();
}

/// Render one TypeScript struct field.
fn render_field(schema: &Schema, scope: &Scope, text: &mut Text, field: &Field, indent: &str) {
    let name = property_key(&field.label());
    let ty = render_property_type(schema, scope, &field.ty);
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
fn render_payload_enum(schema: &Schema, scope: &Scope, text: &mut Text, variants: &[Variant]) {
    for variant in variants {
        text.doc(variant.doc(), "    ");
        text.line("    | {");
        text.line(format!("          readonly kind: {:?};", variant.label()));

        match &variant.payload {
            Payload::Unit => {}
            Payload::Value(ty) => {
                let name = variant.payload_field_name();
                let ty = render_type(schema, scope, ty);

                text.line(format!("          readonly {name}: {ty};"));
            }
            Payload::Struct(fields) => {
                for field in fields {
                    render_payload_field(schema, scope, text, field, "          ");
                }
            }
        }

        text.line("      }");
    }
}

/// Render constructors for one TypeScript payload enum.
fn render_payload_constructors(
    schema: &Schema,
    scope: &Scope,
    text: &mut Text,
    item: &Item,
    variants: &[Variant],
) {
    text.line(format!("export const {} = {{", item.name));

    for variant in variants {
        let method = member(&variant.label());
        let arguments = render_constructor_arguments(schema, scope, variant);
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
fn render_constructor_arguments(schema: &Schema, scope: &Scope, variant: &Variant) -> String {
    constructor_fields(variant)
        .into_iter()
        .map(|(name, ty)| {
            let parameter = identifier(&name);

            format!("{parameter}: {}", render_type(schema, scope, ty))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render TypeScript object fields for one payload constructor.
fn render_constructor_fields(variant: &Variant) -> Vec<String> {
    constructor_fields(variant)
        .into_iter()
        .map(|(name, _)| {
            let property = payload_property_name(&name);
            let parameter = identifier(&name);
            if parameter == property {
                property
            } else {
                format!("{}: {parameter}", property_key(&property))
            }
        })
        .collect()
}

/// Return TypeScript constructor fields for one payload variant.
fn constructor_fields(variant: &Variant) -> Vec<(String, &Type)> {
    match &variant.payload {
        Payload::Unit => Vec::new(),
        Payload::Value(ty) => vec![(variant.payload_field_name(), ty)],
        Payload::Struct(fields) => fields
            .iter()
            .map(|field| (field.label(), &field.ty))
            .collect(),
    }
}

/// Render one payload enum field type.
fn render_payload_field(
    schema: &Schema,
    scope: &Scope,
    text: &mut Text,
    field: &Field,
    indent: &str,
) {
    let name = property_key(&payload_property_name(&field.label()));
    let ty = render_property_type(schema, scope, &field.ty);
    let optional = matches!(field.ty, Type::Option(_));
    let marker = if optional { "?" } else { "" };

    text.doc(field.doc(), indent);
    text.line(format!("{indent}readonly {name}{marker}: {ty};"));
}

/// Render one TypeScript object property type.
fn render_property_type(schema: &Schema, scope: &Scope, ty: &Type) -> String {
    let ty = match ty {
        Type::Option(item) => item,
        _ => ty,
    };

    render_type(schema, scope, ty)
}

/// Render one TypeScript type.
pub(super) fn render_type(schema: &Schema, scope: &Scope, ty: &Type) -> String {
    match ty {
        Type::Unit => "null".to_string(),
        Type::String => "string".to_string(),
        Type::Bool => "boolean".to_string(),
        Type::Char => "string".to_string(),
        Type::U8 | Type::U32 | Type::Usize | Type::Signed(8 | 16 | 32) | Type::Float(_) => {
            "number".to_string()
        }
        Type::U64 | Type::U128 | Type::Signed(64 | 128) => "bigint".to_string(),
        Type::Signed(_) => ty.unsupported(),
        Type::Sequence(ty) if matches!(ty.as_ref(), Type::U8) => {
            "Uint8Array | readonly number[]".to_string()
        }
        Type::Sequence(ty) => format!("ReadonlyArray<{}>", render_type(schema, scope, ty)),
        Type::Option(ty) => format!("{} | undefined", render_type(schema, scope, ty)),
        Type::Array(ty, len) => render_array_type(schema, scope, ty, *len),
        Type::Tuple(types) => {
            let types = types
                .iter()
                .map(|ty| render_type(schema, scope, ty))
                .collect::<Vec<_>>()
                .join(", ");

            format!("readonly [{types}]")
        }
        Type::Map(key, value) if matches!(key.as_ref(), Type::String) => {
            format!(
                "Readonly<Record<string, {}>>",
                render_type(schema, scope, value)
            )
        }
        Type::Map(key, value) => format!(
            "ReadonlyMap<{}, {}>",
            render_type(schema, scope, key),
            render_type(schema, scope, value)
        ),
        Type::Named { .. } => scope.ty(ty),
        Type::Struct(_) | Type::Enum(_) => ty.unsupported(),
    }
}

/// Render one fixed-length array type.
fn render_array_type(schema: &Schema, scope: &Scope, ty: &Type, len: usize) -> String {
    if matches!(ty, Type::U8) {
        "Uint8Array | readonly number[]".to_string()
    } else {
        let ty = render_type(schema, scope, ty);
        let fields = repeat_n(ty, len).collect::<Vec<_>>().join(", ");

        format!("readonly [{fields}]")
    }
}
