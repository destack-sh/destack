use crate::generate::core::to_snake;
use crate::generate::schema::{Field, Item, Payload, Schema, SchemaModule, Shape, Type, Variant};

use super::item::protocol_variant_name;
use super::name::{python_field_name, python_parameter_name, python_payload_field_name};
use super::path::absolute_module_path;
use super::text::Text;

/// Render Python encode and decode functions for one item.
pub(super) fn render_codec_item(schema: &Schema, module: &SchemaModule, item: &Item) -> String {
    let mut text = Text::new();
    let encoder = encode_name(&item.name);
    let decoder = decode_name(&item.name);

    text.line(format!(
        "def {encoder}(writer: BinaryWriter, value: {}) -> None:",
        item.name
    ));
    text.line(format!("    \"\"\"Encode one {}.\"\"\"", item.name));
    render_protocol_encode_item(schema, module, item, &mut text, "value", "    ");
    text.blank();

    text.line(format!(
        "def {decoder}(reader: BinaryReader) -> {}:",
        item.name
    ));
    text.line(format!("    \"\"\"Decode one {}.\"\"\"", item.name));
    render_protocol_decode_item(schema, module, item, &mut text, "    ");
    text.blank();

    text.line(format!(
        "def {}(value: {}) -> Json:",
        to_json_name(&item.name),
        item.name
    ));
    text.line(format!(
        "    \"\"\"Return one JSON value for one {}.\"\"\"",
        item.name
    ));
    render_protocol_to_json_item(schema, module, item, &mut text, "value", "    ");
    text.blank();

    text.line(format!(
        "def {}(value: Json) -> {}:",
        from_json_name(&item.name),
        item.name
    ));
    text.line(format!(
        "    \"\"\"Return one {} from one JSON value.\"\"\"",
        item.name
    ));
    render_protocol_from_json_item(schema, module, item, &mut text, "value", "    ");
    text.blank();

    text.finish()
}

/// Render Python encode and decode stubs for one item.
pub(super) fn render_codec_stub_item(item: &Item) -> String {
    let mut text = Text::new();
    let encoder = encode_name(&item.name);
    let decoder = decode_name(&item.name);

    text.line(format!(
        "def {encoder}(writer: BinaryWriter, value: {}) -> None: ...",
        item.name
    ));
    text.line(format!(
        "def {decoder}(reader: BinaryReader) -> {}: ...",
        item.name
    ));
    text.line(format!(
        "def {}(value: {}) -> Json: ...",
        to_json_name(&item.name),
        item.name
    ));
    text.line(format!(
        "def {}(value: Json) -> {}: ...",
        from_json_name(&item.name),
        item.name
    ));
    text.blank();

    text.finish()
}

/// Render one Python JSON encoder body.
fn render_protocol_to_json_item(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
    text: &mut Text,
    value: &str,
    indent: &str,
) {
    if let Some(ty) = item.scalar_newtype() {
        let value = render_protocol_to_json_type(schema, module, ty, value, 0);

        text.line(format!("{indent}return {value}"));

        return;
    }

    match &item.shape {
        Shape::Struct(fields) => {
            if fields.is_empty() {
                text.line(format!("{indent}return {{}}"));
            } else {
                text.line(format!("{indent}return {{"));
                for field in fields {
                    render_protocol_to_json_field(schema, module, text, field, value, indent);
                }
                text.line(format!("{indent}}}"));
            }
        }
        Shape::Enum(_) if schema.is_unit_enum(&item.key) => {
            text.line(format!("{indent}return {value}"));
        }
        Shape::Enum(variants) => {
            render_protocol_to_json_payload_enum(schema, module, text, variants, value, indent);
        }
    }
}

/// Render one Python JSON decoder body.
fn render_protocol_from_json_item(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
    text: &mut Text,
    value: &str,
    indent: &str,
) {
    if let Some(ty) = item.scalar_newtype() {
        let value = render_protocol_from_json_type(schema, module, ty, value, 0);

        text.line(format!("{indent}return {value}"));

        return;
    }

    match &item.shape {
        Shape::Struct(fields) => {
            text.line(format!("{indent}object_ = json_object({value})"));
            if !fields.is_empty() {
                text.blank();
            }

            text.line(format!("{indent}return {}(", item.name));
            for field in fields {
                render_protocol_from_json_field(schema, module, text, field, indent);
            }
            text.line(format!("{indent})"));
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.key) => {
            render_protocol_from_json_unit_enum(text, variants, value, indent);
        }
        Shape::Enum(variants) => {
            render_protocol_from_json_payload_enum(
                schema, module, item, text, variants, value, indent,
            );
        }
    }
}

/// Render one Python JSON struct field.
fn render_protocol_to_json_field(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    field: &Field,
    value: &str,
    indent: &str,
) {
    let name = python_field_name(&field.name);
    let field_value = format!("{value}.{name}");

    if let Type::Option(item) = &field.ty {
        let json = render_protocol_to_json_type(schema, module, item, &field_value, 0);

        text.line(format!(
            "{indent}    **({{}} if {field_value} is None else {{{:?}: {json}}}),",
            field.label()
        ));
    } else {
        let json = render_protocol_to_json_type(schema, module, &field.ty, &field_value, 0);

        text.line(format!("{indent}    {:?}: {json},", field.label()));
    }
}

/// Render one Python JSON struct field decoder.
fn render_protocol_from_json_field(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    field: &Field,
    indent: &str,
) {
    let name = python_field_name(&field.name);
    let json = if let Type::Option(item) = &field.ty {
        let value = render_protocol_from_json_type(schema, module, item, "value", 0);

        format!(
            "json_optional(object_, {:?}, lambda value: {value})",
            field.label()
        )
    } else {
        let field_value = format!("json_field(object_, {:?})", field.label());

        render_protocol_from_json_type(schema, module, &field.ty, &field_value, 0)
    };

    text.line(format!("{indent}    {name}={json},"));
}

/// Render one Python payload enum JSON encoder.
fn render_protocol_to_json_payload_enum(
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
        text.line(format!("{indent}    return {{"));
        text.line(format!("{indent}        \"kind\": {:?},", variant.label()));

        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(ty) => {
                let field = python_payload_field_name(&variant.payload_field_name());
                let field_value = format!("{value}.{field}");
                let json = render_protocol_to_json_type(schema, module, ty, &field_value, 0);

                text.line(format!(
                    "{indent}        {:?}: {json},",
                    variant.payload_field_name()
                ));
            }
            Payload::Struct(fields) => {
                for field in fields {
                    render_protocol_to_json_payload_field(
                        schema, module, text, field, value, indent,
                    );
                }
            }
        }

        text.line(format!("{indent}    }}"));
    }
    text.line(format!("{indent}else:"));
    text.line(format!(
        "{indent}    raise SerdeError(\"unknown enum variant\")"
    ));
}

/// Render one Python payload enum JSON field.
fn render_protocol_to_json_payload_field(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    field: &Field,
    value: &str,
    indent: &str,
) {
    let name = python_payload_field_name(&field.name);
    let field_value = format!("{value}.{name}");

    if let Type::Option(item) = &field.ty {
        let json = render_protocol_to_json_type(schema, module, item, &field_value, 0);

        text.line(format!(
            "{indent}        **({{}} if {field_value} is None else {{{:?}: {json}}}),",
            field.label()
        ));
    } else {
        let json = render_protocol_to_json_type(schema, module, &field.ty, &field_value, 0);

        text.line(format!("{indent}        {:?}: {json},", field.label()));
    }
}

/// Render one Python unit enum JSON decoder.
fn render_protocol_from_json_unit_enum(
    text: &mut Text,
    variants: &[Variant],
    value: &str,
    indent: &str,
) {
    text.line(format!("{indent}variant = json_string({value})"));
    text.blank();
    for (index, variant) in variants.iter().enumerate() {
        let prefix = if index == 0 { "if" } else { "elif" };
        text.line(format!(
            "{indent}{prefix} variant == {:?}:",
            variant.label()
        ));
        text.line(format!("{indent}    return {:?}", variant.label()));
    }
    text.line(format!("{indent}else:"));
    text.line(format!(
        "{indent}    raise SerdeError(f\"unknown enum variant: {{variant}}\")"
    ));
}

/// Render one Python payload enum JSON decoder.
fn render_protocol_from_json_payload_enum(
    schema: &Schema,
    module: &SchemaModule,
    item: &Item,
    text: &mut Text,
    variants: &[Variant],
    value: &str,
    indent: &str,
) {
    text.line(format!("{indent}object_ = json_object({value})"));
    text.line(format!(
        "{indent}kind = json_string(json_field(object_, \"kind\"))"
    ));
    text.blank();

    for (index, variant) in variants.iter().enumerate() {
        let prefix = if index == 0 { "if" } else { "elif" };
        let class_name = protocol_variant_name(schema, item, variant);
        text.line(format!("{indent}{prefix} kind == {:?}:", variant.label()));

        match &variant.payload {
            Payload::Unit => {
                text.line(format!("{indent}    return {class_name}()"));
            }
            Payload::Tuple(ty) => {
                let field = python_payload_field_name(&variant.payload_field_name());
                let field_value =
                    format!("json_field(object_, {:?})", variant.payload_field_name());
                let json = render_protocol_from_json_type(schema, module, ty, &field_value, 0);

                text.line(format!("{indent}    return {class_name}({field}={json})"));
            }
            Payload::Struct(fields) => {
                text.line(format!("{indent}    return {class_name}("));
                for field in fields {
                    render_protocol_from_json_payload_field(schema, module, text, field, indent);
                }
                text.line(format!("{indent}    )"));
            }
        }
    }
    text.line(format!("{indent}else:"));
    text.line(format!(
        "{indent}    raise SerdeError(f\"unknown enum variant: {{kind}}\")"
    ));
}

/// Render one Python payload enum JSON field decoder.
fn render_protocol_from_json_payload_field(
    schema: &Schema,
    module: &SchemaModule,
    text: &mut Text,
    field: &Field,
    indent: &str,
) {
    let name = python_payload_field_name(&field.name);
    let json = if let Type::Option(item) = &field.ty {
        let value = render_protocol_from_json_type(schema, module, item, "value", 0);

        format!(
            "json_optional(object_, {:?}, lambda value: {value})",
            field.label()
        )
    } else {
        let field_value = format!("json_field(object_, {:?})", field.label());

        render_protocol_from_json_type(schema, module, &field.ty, &field_value, 0)
    };

    text.line(format!("{indent}        {name}={json},"));
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
    if let Some(ty) = item.scalar_newtype() {
        render_protocol_encode_type(schema, module, text, ty, value, indent, 0);

        return;
    }

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
        Shape::Enum(variants) if schema.is_unit_enum(&item.key) => {
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
    if let Some(ty) = item.scalar_newtype() {
        let value = render_protocol_decode_type(schema, module, ty, "reader", 0);

        text.line(format!("{indent}return {value}"));

        return;
    }

    match &item.shape {
        Shape::Struct(fields) => {
            for (index, field) in fields.iter().enumerate() {
                let value = render_protocol_decode_type(schema, module, &field.ty, "reader", index);
                let name = python_field_name(&field.name);
                let local = python_local_name(&name);

                text.line(format!("{indent}{local} = {value}"));
            }
            if !fields.is_empty() {
                text.blank();
            }

            text.line(format!("{indent}return {}(", item.name));
            for field in fields {
                let name = python_field_name(&field.name);
                let local = python_local_name(&name);
                text.line(format!("{indent}    {name}={local},"));
            }
            text.line(format!("{indent})"));
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.key) => {
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
                    let name = python_payload_field_name(&field.name);
                    let field_value = format!("{value}.{name}");
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
        let class_name = protocol_variant_name(schema, item, variant);
        text.line(format!("{indent}{prefix} variant == {index}:"));

        match &variant.payload {
            Payload::Unit => {
                text.line(format!("{indent}    return {class_name}()"));
            }
            Payload::Tuple(ty) => {
                let field = python_payload_field_name(&variant.payload_field_name());
                let local = python_local_name(&field);
                let value = render_protocol_decode_type(schema, module, ty, "reader", 0);
                text.line(format!("{indent}    {local} = {value}"));
                text.blank();
                text.line(format!("{indent}    return {class_name}({field}={local})"));
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
                    let name = python_payload_field_name(&field.name);
                    let local = python_local_name(&name);

                    text.line(format!("{indent}    {local} = {value}"));
                }
                if !fields.is_empty() {
                    text.blank();
                }

                text.line(format!("{indent}    return {class_name}("));
                for field in fields {
                    let name = python_payload_field_name(&field.name);
                    let local = python_local_name(&name);
                    text.line(format!("{indent}        {name}={local},"));
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
        Type::Named { key, .. } => {
            let encoder = protocol_encoder_path(schema, module, key);
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
    let item_name = protocol_local_name("item", value, depth);

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
    let entries = protocol_local_name("entries", value, depth);
    let entry = protocol_local_name("entry", value, depth);
    let key_name = protocol_local_name("key", value, depth);
    let item_name = protocol_local_name("item", value, depth);
    let write_key = protocol_local_name("write_key", value, depth);

    text.line(format!("{indent}{entries} = []"));
    text.line(format!(
        "{indent}for {key_name}, {item_name} in {value}.items():"
    ));
    text.line(format!(
        "{indent}    def {write_key}(writer: BinaryWriter) -> None:"
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
    text.line(format!("{indent}    key_bytes = nested_bytes({write_key})"));
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

/// Render one Python JSON encoder expression.
fn render_protocol_to_json_type(
    schema: &Schema,
    module: &SchemaModule,
    ty: &Type,
    value: &str,
    depth: usize,
) -> String {
    match ty {
        Type::String
        | Type::Bool
        | Type::Char
        | Type::U8
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::Signed(_)
        | Type::Usize
        | Type::Float(_)
        | Type::Json => value.to_string(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            format!("bytes_to_json({value})")
        }
        Type::Vec(item) => render_protocol_to_json_sequence(schema, module, item, value, depth),
        Type::Option(item) => {
            let item = render_protocol_to_json_type(schema, module, item, value, depth);

            format!("None if {value} is None else {item}")
        }
        Type::Array(item, _) if matches!(item.as_ref(), Type::U8) => {
            format!("bytes_to_json({value})")
        }
        Type::Array(item, _) => {
            render_protocol_to_json_sequence(schema, module, item, value, depth)
        }
        Type::Tuple(items) => render_protocol_to_json_tuple(schema, module, items, value, depth),
        Type::Map(key, item) => {
            render_protocol_to_json_map(schema, module, key, item, value, depth)
        }
        Type::Named { key, .. } => {
            let encoder = protocol_json_encoder_path(schema, module, key);

            format!("{encoder}({value})")
        }
    }
}

/// Render one Python JSON decoder expression.
fn render_protocol_from_json_type(
    schema: &Schema,
    module: &SchemaModule,
    ty: &Type,
    value: &str,
    depth: usize,
) -> String {
    match ty {
        Type::String | Type::Char => format!("json_string({value})"),
        Type::Bool => format!("json_bool({value})"),
        Type::U8 | Type::U32 | Type::U64 | Type::U128 | Type::Signed(_) | Type::Usize => {
            format!("json_int({value})")
        }
        Type::Float(_) => format!("json_number({value})"),
        Type::Json => value.to_string(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            format!("bytes_from_json({value})")
        }
        Type::Vec(item) => render_protocol_from_json_sequence(schema, module, item, value, depth),
        Type::Option(item) => {
            let item = render_protocol_from_json_type(schema, module, item, value, depth);

            format!("None if {value} is None else {item}")
        }
        Type::Array(item, _) if matches!(item.as_ref(), Type::U8) => {
            format!("bytes_from_json({value})")
        }
        Type::Array(item, len) => {
            render_protocol_from_json_array(schema, module, item, *len, value, depth)
        }
        Type::Tuple(items) => render_protocol_from_json_tuple(schema, module, items, value, depth),
        Type::Map(key, item) => {
            render_protocol_from_json_map(schema, module, key, item, value, depth)
        }
        Type::Named { key, .. } => {
            let decoder = protocol_json_decoder_path(schema, module, key);

            format!("{decoder}({value})")
        }
    }
}

/// Render one Python JSON sequence encoder expression.
fn render_protocol_to_json_sequence(
    schema: &Schema,
    module: &SchemaModule,
    item: &Type,
    value: &str,
    depth: usize,
) -> String {
    let item_name = format!("item_{depth}");
    let item_value = render_protocol_to_json_type(schema, module, item, &item_name, depth + 1);

    format!("[{item_value} for {item_name} in {value}]")
}

/// Render one Python JSON tuple encoder expression.
fn render_protocol_to_json_tuple(
    schema: &Schema,
    module: &SchemaModule,
    items: &[Type],
    value: &str,
    depth: usize,
) -> String {
    let items = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            render_protocol_to_json_type(
                schema,
                module,
                item,
                &format!("{value}[{index}]"),
                depth + index,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{items}]")
}

/// Render one Python JSON map encoder expression.
fn render_protocol_to_json_map(
    schema: &Schema,
    module: &SchemaModule,
    key: &Type,
    item: &Type,
    value: &str,
    depth: usize,
) -> String {
    let key_name = format!("key_{depth}");
    let item_name = format!("item_{depth}");
    let item_value = render_protocol_to_json_type(schema, module, item, &item_name, depth + 1);

    if matches!(key, Type::String) {
        format!("{{{key_name}: {item_value} for {key_name}, {item_name} in {value}.items()}}")
    } else {
        let key_value = render_protocol_to_json_type(schema, module, key, &key_name, depth + 1);

        format!("[[{key_value}, {item_value}] for {key_name}, {item_name} in {value}.items()]")
    }
}

/// Render one Python JSON sequence decoder expression.
fn render_protocol_from_json_sequence(
    schema: &Schema,
    module: &SchemaModule,
    item: &Type,
    value: &str,
    depth: usize,
) -> String {
    let item_name = format!("item_{depth}");
    let item_value = render_protocol_from_json_type(schema, module, item, &item_name, depth + 1);

    format!("[{item_value} for {item_name} in json_array({value})]")
}

/// Render one Python JSON fixed array decoder expression.
fn render_protocol_from_json_array(
    schema: &Schema,
    module: &SchemaModule,
    item: &Type,
    len: usize,
    value: &str,
    depth: usize,
) -> String {
    let values = (0..len)
        .map(|index| {
            let value = format!("items[{index}]");

            render_protocol_from_json_type(schema, module, item, &value, depth + index)
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("(lambda items: ({values},))(json_array_length({value}, {len}))")
}

/// Render one Python JSON tuple decoder expression.
fn render_protocol_from_json_tuple(
    schema: &Schema,
    module: &SchemaModule,
    items: &[Type],
    value: &str,
    depth: usize,
) -> String {
    let values = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let value = format!("items[{index}]");

            render_protocol_from_json_type(schema, module, item, &value, depth + index)
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "(lambda items: ({values},))(json_array_length({value}, {}))",
        items.len()
    )
}

/// Render one Python JSON map decoder expression.
fn render_protocol_from_json_map(
    schema: &Schema,
    module: &SchemaModule,
    key: &Type,
    item: &Type,
    value: &str,
    depth: usize,
) -> String {
    let key_name = format!("key_{depth}");
    let item_name = format!("item_{depth}");
    let item_value = render_protocol_from_json_type(schema, module, item, &item_name, depth + 1);

    if matches!(key, Type::String) {
        format!(
            "{{{key_name}: {item_value} for {key_name}, {item_name} in json_object({value}).items()}}"
        )
    } else {
        let key_value = render_protocol_from_json_type(schema, module, key, &key_name, depth + 1);

        format!("{{{key_value}: {item_value} for {key_name}, {item_name} in json_array({value})}}")
    }
}

/// Return one stable Python local name for generated encoder code.
fn protocol_local_name(role: &str, value: &str, depth: usize) -> String {
    let mut token = String::new();

    for character in value.chars() {
        // keep generated names readable while avoiding punctuation
        if character.is_ascii_alphanumeric() {
            token.push(character.to_ascii_lowercase());
        } else {
            token.push('_');
        }
    }

    while token.contains("__") {
        token = token.replace("__", "_");
    }

    let token = token.trim_matches('_');

    if token.is_empty() {
        format!("{role}_{depth}")
    } else {
        format!("{role}_{token}_{depth}")
    }
}

/// Return one safe Python local name.
fn python_local_name(name: &str) -> String {
    match name {
        "range" | "reader" | "value" | "writer" => format!("{name}_"),
        _ => name.to_string(),
    }
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
        Type::Named { key, .. } => {
            let decoder = protocol_decoder_path(schema, module, key);

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

/// Return one generated Python JSON encoder name.
pub(super) fn to_json_name(name: &str) -> String {
    format!("to_json_{}", to_snake(name))
}

/// Return one generated Python JSON decoder name.
pub(super) fn from_json_name(name: &str) -> String {
    format!("from_json_{}", to_snake(name))
}

/// Return one Python encoder access path.
fn protocol_encoder_path(schema: &Schema, module: &SchemaModule, name: &str) -> String {
    let item = schema.item(name);

    protocol_codec_path(schema, module, name, &encode_name(&item.name))
}

/// Return one Python decoder access path.
fn protocol_decoder_path(schema: &Schema, module: &SchemaModule, name: &str) -> String {
    let item = schema.item(name);

    protocol_codec_path(schema, module, name, &decode_name(&item.name))
}

/// Return one Python JSON encoder access path.
fn protocol_json_encoder_path(schema: &Schema, module: &SchemaModule, name: &str) -> String {
    let item = schema.item(name);

    protocol_codec_path(schema, module, name, &to_json_name(&item.name))
}

/// Return one Python JSON decoder access path.
fn protocol_json_decoder_path(schema: &Schema, module: &SchemaModule, name: &str) -> String {
    let item = schema.item(name);

    protocol_codec_path(schema, module, name, &from_json_name(&item.name))
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
        format!("{}.{}", absolute_module_path(schema, path), function)
    }
}
