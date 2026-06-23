use crate::generate::core::to_snake;
use crate::generate::schema::{Item, Payload, Schema, SchemaModule, Shape, Type, Variant};

use super::item::protocol_variant_name;
use super::name::{python_field_name, python_parameter_name};
use super::path::protocol_absolute_module_path;
use super::text::Text;

/// Render Python encode and decode functions for one protocol item.
pub(super) fn render_protocol_codec_item(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
) -> String {
    let mut text = Text::new();
    let encoder = encode_name(&item.name);
    let decoder = decode_name(&item.name);

    text.line(format!(
        "def {encoder}(writer: Writer, value: {}) -> None:",
        item.name
    ));
    render_protocol_encode_item(schema, module, item, &mut text, "value", "    ");
    text.blank();

    text.line(format!("def {decoder}(reader: Reader) -> {}:", item.name));
    render_protocol_decode_item(schema, module, item, &mut text, "    ");
    text.blank();

    text.finish()
}

/// Render Python encode and decode stubs for one protocol item.
pub(super) fn render_protocol_codec_stub_item(item: &Item) -> String {
    let mut text = Text::new();
    let encoder = encode_name(&item.name);
    let decoder = decode_name(&item.name);

    text.line(format!(
        "def {encoder}(writer: Writer, value: {}) -> None: ...",
        item.name
    ));
    text.line(format!(
        "def {decoder}(reader: Reader) -> {}: ...",
        item.name
    ));
    text.blank();

    text.finish()
}

/// Render one Python protocol encoder body.
fn render_protocol_encode_item(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
    text: &mut Text,
    value: &str,
    indent: &str,
) {
    match &item.shape {
        Shape::Struct(fields) => {
            if fields.is_empty() {
                text.line(format!("{indent}pass"));
            } else {
                for field in fields {
                    let field_value = format!("{value}.{}", python_field_name(&field.name));
                    render_protocol_encode_type(
                        schema,
                        module,
                        text,
                        &field.ty,
                        &field_value,
                        indent,
                        0,
                    );
                }
            }
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.name) => {
            render_protocol_encode_unit_enum(text, variants, value, indent);
        }
        Shape::Enum(variants) => {
            render_protocol_encode_payload_enum(schema, module, text, variants, value, indent);
        }
    }
}

/// Render one Python protocol decoder body.
fn render_protocol_decode_item(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
    text: &mut Text,
    indent: &str,
) {
    match &item.shape {
        Shape::Struct(fields) => {
            for (index, field) in fields.iter().enumerate() {
                let value = render_protocol_decode_type(schema, module, &field.ty, "reader", index);
                text.line(format!("{indent}field_{index} = {value}"));
            }
            if !fields.is_empty() {
                text.blank();
            }

            text.line(format!("{indent}return {}(", item.name));
            for (index, field) in fields.iter().enumerate() {
                let name = python_field_name(&field.name);
                text.line(format!("{indent}    {name}=field_{index},"));
            }
            text.line(format!("{indent})"));
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.name) => {
            render_protocol_decode_unit_enum(text, variants, indent);
        }
        Shape::Enum(variants) => {
            render_protocol_decode_payload_enum(schema, module, item, text, variants, indent);
        }
    }
}

/// Render one Python unit enum encoder.
fn render_protocol_encode_unit_enum(
    text: &mut Text,
    variants: &[Variant],
    value: &str,
    indent: &str,
) {
    for (index, variant) in variants.iter().enumerate() {
        let prefix = if index == 0 { "if" } else { "elif" };
        text.line(format!(
            "{indent}{prefix} {value} == {:?}:",
            variant.label()
        ));
        text.line(format!("{indent}    writer.write_unsigned({index})"));
    }
    text.line(format!("{indent}else:"));
    text.line(format!(
        "{indent}    raise SerdeError(\"unknown enum variant\")"
    ));
}

/// Render one Python unit enum decoder.
fn render_protocol_decode_unit_enum(text: &mut Text, variants: &[Variant], indent: &str) {
    text.line(format!("{indent}variant = reader.read_number()"));
    text.blank();
    for (index, variant) in variants.iter().enumerate() {
        let prefix = if index == 0 { "if" } else { "elif" };
        text.line(format!("{indent}{prefix} variant == {index}:"));
        text.line(format!("{indent}    return {:?}", variant.label()));
    }
    text.line(format!("{indent}else:"));
    text.line(format!(
        "{indent}    raise SerdeError(f\"unknown enum variant index: {{variant}}\")"
    ));
}

/// Render one Python payload enum encoder.
fn render_protocol_encode_payload_enum(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    variants: &[Variant],
    value: &str,
    indent: &str,
) {
    for (index, variant) in variants.iter().enumerate() {
        let prefix = if index == 0 { "if" } else { "elif" };
        text.line(format!(
            "{indent}{prefix} {value}.kind == {:?}:",
            variant.label()
        ));
        text.line(format!("{indent}    writer.write_unsigned({index})"));

        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(ty) => {
                let field = python_parameter_name(&variant.payload_field_name());
                let field = format!("{value}.{field}");
                render_protocol_encode_type(
                    schema,
                    module,
                    text,
                    ty,
                    &field,
                    &format!("{indent}    "),
                    0,
                );
            }
            Payload::Struct(fields) => {
                for field in fields {
                    let field_value = format!("{value}.{}", python_field_name(&field.name));
                    render_protocol_encode_type(
                        schema,
                        module,
                        text,
                        &field.ty,
                        &field_value,
                        &format!("{indent}    "),
                        0,
                    );
                }
            }
        }
    }
    text.line(format!("{indent}else:"));
    text.line(format!(
        "{indent}    raise SerdeError(\"unknown enum variant\")"
    ));
}

/// Render one Python payload enum decoder.
fn render_protocol_decode_payload_enum(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
    text: &mut Text,
    variants: &[Variant],
    indent: &str,
) {
    text.line(format!("{indent}variant = reader.read_number()"));
    text.blank();

    for (index, variant) in variants.iter().enumerate() {
        let prefix = if index == 0 { "if" } else { "elif" };
        let class_name = protocol_variant_name(item, variant);
        text.line(format!("{indent}{prefix} variant == {index}:"));

        match &variant.payload {
            Payload::Unit => {
                text.line(format!("{indent}    return {class_name}()"));
            }
            Payload::Tuple(ty) => {
                let field = python_parameter_name(&variant.payload_field_name());
                let value = render_protocol_decode_type(schema, module, ty, "reader", 0);
                text.line(format!("{indent}    return {class_name}({field}={value})"));
            }
            Payload::Struct(fields) => {
                for (field_index, field) in fields.iter().enumerate() {
                    let value = render_protocol_decode_type(
                        schema,
                        module,
                        &field.ty,
                        "reader",
                        field_index,
                    );
                    text.line(format!("{indent}    field_{field_index} = {value}"));
                }
                if !fields.is_empty() {
                    text.blank();
                }

                text.line(format!("{indent}    return {class_name}("));
                for (field_index, field) in fields.iter().enumerate() {
                    let name = python_field_name(&field.name);
                    text.line(format!("{indent}        {name}=field_{field_index},"));
                }
                text.line(format!("{indent}    )"));
            }
        }
    }
    text.line(format!("{indent}else:"));
    text.line(format!(
        "{indent}    raise SerdeError(f\"unknown enum variant index: {{variant}}\")"
    ));
}

/// Render one Python protocol value encoder.
fn render_protocol_encode_type(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    ty: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    match ty {
        Type::String => text.line(format!("{indent}writer.write_string({value})")),
        Type::Bool => text.line(format!("{indent}writer.write_bool({value})")),
        Type::Char => text.line(format!("{indent}writer.write_char({value})")),
        Type::U8 => text.line(format!("{indent}writer.write_byte({value})")),
        Type::U32 | Type::U64 | Type::U128 | Type::Usize => {
            text.line(format!("{indent}writer.write_unsigned({value})"));
        }
        Type::Signed(8) => text.line(format!("{indent}writer.write_i8({value})")),
        Type::Signed(_) => text.line(format!("{indent}writer.write_signed({value})")),
        Type::Float(32) => text.line(format!("{indent}writer.write_f32({value})")),
        Type::Float(64) => text.line(format!("{indent}writer.write_f64({value})")),
        Type::Float(_) => ty.unsupported_bridge_type(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            text.line(format!("{indent}writer.write_byte_slice({value})"));
        }
        Type::Vec(item) => {
            render_protocol_encode_sequence(schema, module, text, item, value, indent, depth)
        }
        Type::Option(item) => {
            render_protocol_encode_option(schema, module, text, item, value, indent, depth)
        }
        Type::Array(item, _) => {
            render_protocol_encode_array(schema, module, text, item, value, indent, depth);
        }
        Type::Tuple(items) => {
            render_protocol_encode_tuple(schema, module, text, items, value, indent, depth);
        }
        Type::Map(key, item) => {
            render_protocol_encode_map(schema, module, text, key, item, value, indent, depth);
        }
        Type::Json => text.line(format!("{indent}writer.write_json({value})")),
        Type::Named(name) => {
            let encoder = protocol_encoder_path(schema, module, name);
            text.line(format!("{indent}{encoder}(writer, {value})"));
        }
    }
}

/// Render one Python option encoder.
fn render_protocol_encode_option(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    item: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    text.line(format!("{indent}if {value} is None:"));
    text.line(format!("{indent}    writer.write_byte(0)"));
    text.line(format!("{indent}else:"));
    text.line(format!("{indent}    writer.write_byte(1)"));
    render_protocol_encode_type(
        schema,
        module,
        text,
        item,
        value,
        &format!("{indent}    "),
        depth + 1,
    );
}

/// Render one Python sequence encoder.
fn render_protocol_encode_sequence(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    item: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    let item_name = format!("item_{depth}");
    text.line(format!("{indent}writer.write_unsigned(len({value}))"));
    text.line(format!("{indent}for {item_name} in {value}:"));
    render_protocol_encode_type(
        schema,
        module,
        text,
        item,
        &item_name,
        &format!("{indent}    "),
        depth + 1,
    );
}

/// Render one Python fixed array encoder.
fn render_protocol_encode_array(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    item: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    if matches!(item, Type::U8) {
        text.line(format!("{indent}writer.write_bytes({value})"));
        return;
    }

    render_protocol_encode_sequence(schema, module, text, item, value, indent, depth);
}

/// Render one Python tuple encoder.
fn render_protocol_encode_tuple(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    items: &[Type],
    value: &str,
    indent: &str,
    depth: usize,
) {
    for (index, item) in items.iter().enumerate() {
        render_protocol_encode_type(
            schema,
            module,
            text,
            item,
            &format!("{value}[{index}]"),
            indent,
            depth + index,
        );
    }
}

/// Render one Python map encoder.
fn render_protocol_encode_map(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    key: &Type,
    item: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    let entries = format!("entries_{depth}");
    let entry = format!("entry_{depth}");
    let key_name = format!("key_{depth}");
    let item_name = format!("item_{depth}");

    text.line(format!("{indent}{entries} = []"));
    text.line(format!(
        "{indent}for {key_name}, {item_name} in {value}.items():"
    ));
    text.line(format!(
        "{indent}    def write_key(writer: Writer) -> None:"
    ));
    render_protocol_encode_type(
        schema,
        module,
        text,
        key,
        &key_name,
        &format!("{indent}        "),
        depth + 1,
    );
    text.line(format!("{indent}    key_bytes = nested_bytes(write_key)"));
    text.line(format!(
        "{indent}    {entries}.append(({key_name}, {item_name}, key_bytes))"
    ));
    text.line(format!(
        "{indent}{entries}.sort(key=lambda entry: entry[2])"
    ));
    text.line(format!("{indent}writer.write_unsigned(len({entries}))"));
    text.line(format!("{indent}for {entry} in {entries}:"));
    render_protocol_encode_type(
        schema,
        module,
        text,
        key,
        &format!("{entry}[0]"),
        &format!("{indent}    "),
        depth + 1,
    );
    render_protocol_encode_type(
        schema,
        module,
        text,
        item,
        &format!("{entry}[1]"),
        &format!("{indent}    "),
        depth + 1,
    );
}

/// Render one Python protocol value decoder expression.
fn render_protocol_decode_type(
    schema: &Schema,
    module: &SchemaModule,
    ty: &Type,
    reader: &str,
    depth: usize,
) -> String {
    match ty {
        Type::String => format!("{reader}.read_string()"),
        Type::Bool => format!("{reader}.read_bool()"),
        Type::Char => format!("{reader}.read_char()"),
        Type::U8 => format!("{reader}.read_byte()"),
        Type::U32 | Type::U64 | Type::Usize => format!("{reader}.read_number()"),
        Type::U128 => format!("{reader}.read_unsigned()"),
        Type::Signed(8) => format!("{reader}.read_i8()"),
        Type::Signed(_) => format!("{reader}.read_signed_number()"),
        Type::Float(32) => format!("{reader}.read_f32()"),
        Type::Float(64) => format!("{reader}.read_f64()"),
        Type::Float(_) => ty.unsupported_bridge_type(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            format!("{reader}.read_byte_slice()")
        }
        Type::Vec(item) => render_protocol_decode_sequence(schema, module, item, reader, depth),
        Type::Option(item) => {
            let value = render_protocol_decode_type(schema, module, item, reader, depth + 1);

            format!("{reader}.read_option(lambda: {value})")
        }
        Type::Array(item, len) => {
            render_protocol_decode_array(schema, module, item, *len, reader, depth)
        }
        Type::Tuple(items) => render_protocol_decode_tuple(schema, module, items, reader, depth),
        Type::Map(key, item) => {
            render_protocol_decode_map(schema, module, key, item, reader, depth)
        }
        Type::Json => format!("{reader}.read_json()"),
        Type::Named(name) => {
            let decoder = protocol_decoder_path(schema, module, name);

            format!("{decoder}({reader})")
        }
    }
}

/// Render one Python sequence decoder expression.
fn render_protocol_decode_sequence(
    schema: &Schema,
    module: &SchemaModule,
    item: &Type,
    reader: &str,
    depth: usize,
) -> String {
    let value = render_protocol_decode_type(schema, module, item, reader, depth + 1);

    format!("[{value} for _ in range({reader}.read_number())]")
}

/// Render one Python fixed array decoder expression.
fn render_protocol_decode_array(
    schema: &Schema,
    module: &SchemaModule,
    item: &Type,
    len: usize,
    reader: &str,
    depth: usize,
) -> String {
    if matches!(item, Type::U8) {
        return format!("{reader}.read_bytes({len})");
    }

    let items = (0..len)
        .map(|index| render_protocol_decode_type(schema, module, item, reader, depth + index))
        .collect::<Vec<_>>()
        .join(", ");

    format!("({items},)")
}

/// Render one Python tuple decoder expression.
fn render_protocol_decode_tuple(
    schema: &Schema,
    module: &SchemaModule,
    items: &[Type],
    reader: &str,
    depth: usize,
) -> String {
    let items = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            render_protocol_decode_type(schema, module, item, reader, depth + index)
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("({items},)")
}

/// Render one Python map decoder expression.
fn render_protocol_decode_map(
    schema: &Schema,
    module: &SchemaModule,
    key: &Type,
    item: &Type,
    reader: &str,
    depth: usize,
) -> String {
    let key_value = render_protocol_decode_type(schema, module, key, reader, depth + 1);
    let item_value = render_protocol_decode_type(schema, module, item, reader, depth + 2);

    format!("{{{key_value}: {item_value} for _ in range({reader}.read_number())}}")
}

/// Return one generated Python encoder name.
pub(super) fn encode_name(name: &str) -> String {
    format!("encode_{}", to_snake(name))
}

/// Return one generated Python decoder name.
pub(super) fn decode_name(name: &str) -> String {
    format!("decode_{}", to_snake(name))
}

/// Return one Python encoder access path.
fn protocol_encoder_path(schema: &Schema, module: &SchemaModule, name: &str) -> String {
    protocol_codec_path(schema, module, name, &encode_name(name))
}

/// Return one Python decoder access path.
fn protocol_decoder_path(schema: &Schema, module: &SchemaModule, name: &str) -> String {
    protocol_codec_path(schema, module, name, &decode_name(name))
}

/// Return one Python codec function access path.
fn protocol_codec_path(
    schema: &Schema,
    module: &SchemaModule,
    name: &str,
    function: &str,
) -> String {
    let path = schema.module_path(name);
    if path == &module.path {
        function.to_string()
    } else {
        format!(
            "{}.{}",
            protocol_absolute_module_path(schema, path),
            function
        )
    }
}
