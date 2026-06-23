use crate::generate::schema::{Field, Item, Payload, Schema, Shape, Type, Variant};

use super::text::Text;

/// Render one bridge item as a TypeScript type.
pub(super) fn render_item(schema: &Schema, item: &Item, text: &mut Text) {
    ItemRenderer { schema, item }.render(text);
}

/// TypeScript item renderer.
struct ItemRenderer<'schema> {
    /// Bridge schema.
    schema: &'schema Schema,
    /// Bridge item.
    item: &'schema Item,
}

impl<'schema> ItemRenderer<'schema> {
    /// Render this TypeScript item.
    fn render(&self, text: &mut Text) {
        let item = self.item;
        text.doc(item.doc(), "");

        match &item.shape {
            Shape::Struct(fields) => {
                text.line(format!("export type {} = {{", item.name));
                for field in fields {
                    render_field(self.schema, text, field, "    ");
                }
                text.line("};");
                text.blank();
            }
            Shape::Enum(variants) if self.schema.is_unit_enum(&item.name) => {
                text.line(format!(
                    "export type {} = {};",
                    item.name,
                    render_unit_enum(variants)
                ));
                text.blank();
            }
            Shape::Enum(variants) => {
                text.line(format!("export type {} =", item.name));
                render_payload_enum(self.schema, text, variants);
                text.line(";");
                text.blank();
                render_payload_constructors(self.schema, text, item, variants);
            }
        }
    }
}

/// One generated text document.
fn render_field(schema: &Schema, text: &mut Text, field: &Field, indent: &str) {
    let name = field.label();
    let ty = render_field_type(schema, &field.ty);
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
fn render_payload_enum(schema: &Schema, text: &mut Text, variants: &[Variant]) {
    for variant in variants {
        text.doc(variant.doc(), "    ");
        text.line("    | {");
        text.line(format!("          readonly kind: {:?};", variant.label()));

        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(ty) => {
                let name = variant.payload_field_name();
                let ty = render_type(schema, ty);

                text.line(format!("          readonly {name}: {ty};"));
            }
            Payload::Struct(fields) => {
                for field in fields {
                    render_field(schema, text, field, "          ");
                }
            }
        }

        text.line("      }");
    }
}

/// Render constructors for one TypeScript payload enum.
fn render_payload_constructors(
    schema: &Schema,
    text: &mut Text,
    item: &Item,
    variants: &[Variant],
) {
    text.line(format!("export const {} = {{", item.name));

    for variant in variants {
        let method = variant.label();
        let arguments = render_constructor_arguments(schema, variant);
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

    text.line("};");
    text.blank();
}

/// Render TypeScript constructor arguments for one payload variant.
fn render_constructor_arguments(schema: &Schema, variant: &Variant) -> String {
    constructor_fields(variant)
        .into_iter()
        .map(|(name, ty)| {
            let parameter = ts_parameter_name(&name);

            format!("{parameter}: {}", render_type(schema, &ty))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render TypeScript object fields for one payload constructor.
fn render_constructor_fields(variant: &Variant) -> Vec<String> {
    constructor_fields(variant)
        .into_iter()
        .map(|(name, _ty)| {
            let parameter = ts_parameter_name(&name);
            if parameter == name {
                name
            } else {
                format!("{name}: {parameter}")
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
    match name {
        "arguments" => "argumentValues".to_string(),
        "package" => "packageValue".to_string(),
        _ => name.to_string(),
    }
}

/// Render one TypeScript field type.
fn render_field_type(schema: &Schema, ty: &Type) -> String {
    match ty {
        Type::Option(ty) => render_type(schema, ty),
        _ => render_type(schema, ty),
    }
}

/// Render one TypeScript type.
pub(super) fn render_type(schema: &Schema, ty: &Type) -> String {
    match ty {
        Type::String => "string".to_string(),
        Type::Bool => "boolean".to_string(),
        Type::Char => "string".to_string(),
        Type::U8 | Type::U32 | Type::U64 | Type::Usize | Type::Signed(_) | Type::Float(_) => {
            "number".to_string()
        }
        Type::U128 => "bigint".to_string(),
        Type::Vec(ty) if matches!(ty.as_ref(), Type::U8) => {
            "Uint8Array | readonly number[]".to_string()
        }
        Type::Vec(ty) => format!("ReadonlyArray<{}>", render_type(schema, ty)),
        Type::Option(ty) => format!("{} | undefined", render_type(schema, ty)),
        Type::Array(ty, len) => render_array_type(schema, ty, *len),
        Type::Tuple(types) => {
            let types = types
                .iter()
                .map(|ty| render_type(schema, ty))
                .collect::<Vec<_>>()
                .join(", ");

            format!("readonly [{types}]")
        }
        Type::Map(key, value) if matches!(key.as_ref(), Type::String) => {
            format!("Readonly<Record<string, {}>>", render_type(schema, value))
        }
        Type::Map(key, value) => format!(
            "ReadonlyMap<{}, {}>",
            render_type(schema, key),
            render_type(schema, value)
        ),
        Type::Json => "unknown".to_string(),
        Type::Named(name) if schema.is_unit_enum(name) => name.clone(),
        Type::Named(name) => name.clone(),
    }
}

/// Render one fixed-length array type.
fn render_array_type(schema: &Schema, ty: &Type, len: usize) -> String {
    if matches!(ty, Type::U8) {
        return "Uint8Array | readonly number[]".to_string();
    }

    let ty = render_type(schema, ty);
    let fields = std::iter::repeat_n(ty, len).collect::<Vec<_>>().join(", ");

    format!("readonly [{fields}]")
}
