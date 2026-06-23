use crate::generate::schema::{Field, Item, Payload, Schema, Shape, Type, Variant};

use super::item::render_type;
use super::text::Text;

/// Render encode and decode functions for one protocol item.
pub(super) fn render_codec_item(schema: &Schema, item: &Item, text: &mut Text) {
    text.line(format!(
        "export function {}(writer: Writer, value: {}): void {{",
        encode_name(&item.name),
        item.name
    ));
    render_encode_item(schema, item, text, "value", "    ");
    text.line("}");
    text.blank();

    text.line(format!(
        "export function {}(reader: Reader): {} {{",
        decode_name(&item.name),
        item.name
    ));
    render_decode_item(schema, item, text, "    ");
    text.line("}");
    text.blank();
}

/// Render one item encoder body.
fn render_encode_item(schema: &Schema, item: &Item, text: &mut Text, value: &str, indent: &str) {
    match &item.shape {
        Shape::Struct(fields) => {
            for field in fields {
                let field_value = format!("{value}[{}]", property_key(&field.label()));
                render_encode_type(schema, text, &field.ty, &field_value, indent, 0);
            }
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.name) => {
            render_encode_unit_enum(text, variants, value, indent);
        }
        Shape::Enum(variants) => {
            render_encode_payload_enum(schema, text, variants, value, indent);
        }
    }
}

/// Render one item decoder body.
fn render_decode_item(schema: &Schema, item: &Item, text: &mut Text, indent: &str) {
    match &item.shape {
        Shape::Struct(fields) => {
            for (index, field) in fields.iter().enumerate() {
                let value = render_decode_type(schema, &field.ty, "reader", index);
                text.line(format!("{indent}const field{index} = {value};"));
            }
            if !fields.is_empty() {
                text.blank();
            }

            text.line(format!("{indent}return {{"));
            for (index, field) in fields.iter().enumerate() {
                render_decoded_field(text, field, index, indent);
            }
            text.line(format!("{indent}}};"));
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.name) => {
            render_decode_unit_enum(text, variants, indent);
        }
        Shape::Enum(variants) => {
            render_decode_payload_enum(schema, text, variants, indent);
        }
    }
}

/// Render one unit enum encoder.
fn render_encode_unit_enum(text: &mut Text, variants: &[Variant], value: &str, indent: &str) {
    text.line(format!("{indent}switch ({value}) {{"));
    for (index, variant) in variants.iter().enumerate() {
        text.line(format!("{indent}    case {:?}:", variant.label()));
        text.line(format!("{indent}        writer.writeUnsigned({index});"));
        text.line(format!("{indent}        return;"));
    }
    text.line(format!("{indent}}}"));
    text.blank();
    text.line(format!(
        "{indent}throw new SerdeError(\"unknown enum variant\");"
    ));
}

/// Render one unit enum decoder.
fn render_decode_unit_enum(text: &mut Text, variants: &[Variant], indent: &str) {
    text.line(format!("{indent}const variant = reader.readNumber();"));
    text.blank();
    text.line(format!("{indent}switch (variant) {{"));
    for (index, variant) in variants.iter().enumerate() {
        text.line(format!("{indent}    case {index}:"));
        text.line(format!("{indent}        return {:?};", variant.label()));
    }
    text.line(format!("{indent}}}"));
    text.blank();
    text.line(format!(
        "{indent}throw new SerdeError(`unknown enum variant index: ${{variant}}`);"
    ));
}

/// Render one payload enum encoder.
fn render_encode_payload_enum(
    schema: &Schema,
    text: &mut Text,
    variants: &[Variant],
    value: &str,
    indent: &str,
) {
    text.line(format!("{indent}switch ({value}.kind) {{"));
    for (index, variant) in variants.iter().enumerate() {
        text.line(format!("{indent}    case {:?}:", variant.label()));
        text.line(format!("{indent}        writer.writeUnsigned({index});"));
        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(ty) => {
                let field = format!("{value}[{}]", property_key(&variant.payload_field_name()));
                render_encode_type(schema, text, ty, &field, &format!("{indent}        "), 0);
            }
            Payload::Struct(fields) => {
                for field in fields {
                    let field_value = format!("{value}[{}]", property_key(&field.label()));
                    render_encode_type(
                        schema,
                        text,
                        &field.ty,
                        &field_value,
                        &format!("{indent}        "),
                        0,
                    );
                }
            }
        }
        text.line(format!("{indent}        return;"));
    }
    text.line(format!("{indent}}}"));
    text.blank();
    text.line(format!(
        "{indent}throw new SerdeError(\"unknown enum variant\");"
    ));
}

/// Render one payload enum decoder.
fn render_decode_payload_enum(
    schema: &Schema,
    text: &mut Text,
    variants: &[Variant],
    indent: &str,
) {
    text.line(format!("{indent}const variant = reader.readNumber();"));
    text.blank();
    text.line(format!("{indent}switch (variant) {{"));
    for (index, variant) in variants.iter().enumerate() {
        text.line(format!("{indent}    case {index}: {{"));
        match &variant.payload {
            Payload::Unit => {
                text.line(format!(
                    "{indent}        return {{ kind: {:?} }};",
                    variant.label()
                ));
            }
            Payload::Tuple(ty) => {
                let value = render_decode_type(schema, ty, "reader", 0);
                text.line(format!(
                    "{indent}        return {{ kind: {:?}, {}: {value} }};",
                    variant.label(),
                    property_key(&variant.payload_field_name())
                ));
            }
            Payload::Struct(fields) => {
                for (field_index, field) in fields.iter().enumerate() {
                    let value = render_decode_type(schema, &field.ty, "reader", field_index);
                    text.line(format!(
                        "{indent}        const field{field_index} = {value};"
                    ));
                }
                if !fields.is_empty() {
                    text.blank();
                }
                text.line(format!("{indent}        return {{"));
                text.line(format!("{indent}            kind: {:?},", variant.label()));
                for (field_index, field) in fields.iter().enumerate() {
                    render_decoded_field(text, field, field_index, &format!("{indent}        "));
                }
                text.line(format!("{indent}        }};"));
            }
        }
        text.line(format!("{indent}    }}"));
    }
    text.line(format!("{indent}}}"));
    text.blank();
    text.line(format!(
        "{indent}throw new SerdeError(`unknown enum variant index: ${{variant}}`);"
    ));
}

/// Render one decoded struct field.
fn render_decoded_field(text: &mut Text, field: &Field, index: usize, indent: &str) {
    let key = property_key(&field.label());

    if matches!(field.ty, Type::Option(_)) {
        text.line(format!(
            "{indent}    ...(field{index} === undefined ? {{}} : {{ {key}: field{index} }}),"
        ));
    } else {
        text.line(format!("{indent}    {key}: field{index},"));
    }
}

/// Render one value encoder.
fn render_encode_type(
    schema: &Schema,
    text: &mut Text,
    ty: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    match ty {
        Type::String => text.line(format!("{indent}writer.writeString({value});")),
        Type::Bool => text.line(format!("{indent}writer.writeBool({value});")),
        Type::Char => text.line(format!("{indent}writer.writeChar({value});")),
        Type::U8 => text.line(format!("{indent}writer.writeByte({value});")),
        Type::U32 | Type::U64 | Type::U128 | Type::Usize => {
            text.line(format!("{indent}writer.writeUnsigned({value});"));
        }
        Type::Signed(8) => text.line(format!("{indent}writer.writeI8({value});")),
        Type::Signed(_) => text.line(format!("{indent}writer.writeSigned({value});")),
        Type::Float(32) => text.line(format!("{indent}writer.writeF32({value});")),
        Type::Float(64) => text.line(format!("{indent}writer.writeF64({value});")),
        Type::Float(_) => ty.unsupported_bridge_type(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            text.line(format!("{indent}writer.writeByteSlice({value});"));
        }
        Type::Vec(item) => render_encode_sequence(schema, text, item, value, indent, depth),
        Type::Option(item) => {
            text.line(format!(
                "{indent}writer.writeOption({value}, (value{depth}) => {{"
            ));
            render_encode_type(
                schema,
                text,
                item,
                &format!("value{depth}"),
                &format!("{indent}    "),
                depth + 1,
            );
            text.line(format!("{indent}}});"));
        }
        Type::Array(item, _) => render_encode_array(schema, text, item, value, indent, depth),
        Type::Tuple(items) => render_encode_tuple(schema, text, items, value, indent, depth),
        Type::Map(key, item) => render_encode_map(schema, text, key, item, value, indent, depth),
        Type::Json => text.line(format!("{indent}writer.writeJson({value});")),
        Type::Named(name) => {
            text.line(format!("{}{}(writer, {value});", indent, encode_name(name)))
        }
    }
}

/// Render one sequence encoder.
fn render_encode_sequence(
    schema: &Schema,
    text: &mut Text,
    item: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    let item_name = format!("item{depth}");
    text.line(format!("{indent}writer.writeUnsigned({value}.length);"));
    text.line(format!("{indent}for (const {item_name} of {value}) {{"));
    render_encode_type(
        schema,
        text,
        item,
        &item_name,
        &format!("{indent}    "),
        depth + 1,
    );
    text.line(format!("{indent}}}"));
}

/// Render one fixed array encoder.
fn render_encode_array(
    schema: &Schema,
    text: &mut Text,
    item: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    if matches!(item, Type::U8) {
        text.line(format!("{indent}writer.writeBytes({value});"));
        return;
    }

    let item_name = format!("item{depth}");
    text.line(format!("{indent}for (const {item_name} of {value}) {{"));
    render_encode_type(
        schema,
        text,
        item,
        &item_name,
        &format!("{indent}    "),
        depth + 1,
    );
    text.line(format!("{indent}}}"));
}

/// Render one tuple encoder.
fn render_encode_tuple(
    schema: &Schema,
    text: &mut Text,
    items: &[Type],
    value: &str,
    indent: &str,
    depth: usize,
) {
    for (index, item) in items.iter().enumerate() {
        render_encode_type(
            schema,
            text,
            item,
            &format!("{value}[{index}]"),
            indent,
            depth + index,
        );
    }
}

/// Render one map encoder.
fn render_encode_map(
    schema: &Schema,
    text: &mut Text,
    key: &Type,
    item: &Type,
    value: &str,
    indent: &str,
    depth: usize,
) {
    let entries = format!("entries{depth}");
    let entry = format!("entry{depth}");
    let key_name = format!("key{depth}");
    let item_name = format!("item{depth}");

    if matches!(key, Type::String) {
        text.line(format!("{indent}const {entries} = Object.entries({value}).map(([{key_name}, {item_name}]) => {{"));
    } else {
        text.line(format!("{indent}const {entries} = Array.from({value}.entries()).map(([{key_name}, {item_name}]) => {{"));
    }
    text.line(format!(
        "{indent}    const keyBytes = nestedBytes((writer) => {{"
    ));
    render_encode_type(
        schema,
        text,
        key,
        &key_name,
        &format!("{indent}        "),
        depth + 1,
    );
    text.line(format!("{indent}    }});"));
    text.line(format!(
        "{indent}    return {{ {key_name}, {item_name}, keyBytes }};"
    ));
    text.line(format!("{indent}}});"));
    text.line(format!(
        "{indent}{entries}.sort((left, right) => compareBytes(left.keyBytes, right.keyBytes));"
    ));
    text.line(format!("{indent}writer.writeUnsigned({entries}.length);"));
    text.line(format!("{indent}for (const {entry} of {entries}) {{"));
    render_encode_type(
        schema,
        text,
        key,
        &format!("{entry}.{key_name}"),
        &format!("{indent}    "),
        depth + 1,
    );
    render_encode_type(
        schema,
        text,
        item,
        &format!("{entry}.{item_name}"),
        &format!("{indent}    "),
        depth + 1,
    );
    text.line(format!("{indent}}}"));
}

/// Render one value decoder expression.
fn render_decode_type(schema: &Schema, ty: &Type, reader: &str, depth: usize) -> String {
    match ty {
        Type::String => format!("{reader}.readString()"),
        Type::Bool => format!("{reader}.readBool()"),
        Type::Char => format!("{reader}.readChar()"),
        Type::U8 => format!("{reader}.readByte()"),
        Type::U32 | Type::U64 | Type::Usize => format!("{reader}.readNumber()"),
        Type::U128 => format!("{reader}.readUnsigned()"),
        Type::Signed(8) => format!("{reader}.readI8()"),
        Type::Signed(_) => format!("{reader}.readSignedNumber()"),
        Type::Float(32) => format!("{reader}.readF32()"),
        Type::Float(64) => format!("{reader}.readF64()"),
        Type::Float(_) => ty.unsupported_bridge_type(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => format!("{reader}.readByteSlice()"),
        Type::Vec(item) => render_decode_sequence(schema, item, reader, depth),
        Type::Option(item) => {
            let value = render_decode_type(schema, item, reader, depth + 1);

            format!("{reader}.readOption(() => {value})")
        }
        Type::Array(item, len) => render_decode_array(schema, item, *len, reader, depth),
        Type::Tuple(items) => render_decode_tuple(schema, items, reader, depth),
        Type::Map(key, item) => render_decode_map(schema, key, item, reader, depth),
        Type::Json => format!("{reader}.readJson()"),
        Type::Named(name) => format!("{}({reader})", decode_name(name)),
    }
}

/// Render one sequence decoder expression.
fn render_decode_sequence(schema: &Schema, item: &Type, reader: &str, depth: usize) -> String {
    let len = format!("length{depth}");
    let out = format!("items{depth}");
    let value = render_decode_type(schema, item, reader, depth + 1);

    let item = render_type(schema, item);

    format!(
        "(() => {{ const {len} = {reader}.readNumber(); const {out}: Array<{item}> = []; for (let index = 0; index < {len}; index += 1) {{ {out}.push({value}); }} return {out}; }})()"
    )
}

/// Render one fixed array decoder expression.
fn render_decode_array(
    schema: &Schema,
    item: &Type,
    len: usize,
    reader: &str,
    depth: usize,
) -> String {
    if matches!(item, Type::U8) {
        return format!("{reader}.readBytes({len})");
    }

    let items = (0..len)
        .map(|index| render_decode_type(schema, item, reader, depth + index))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{items}]")
}

/// Render one tuple decoder expression.
fn render_decode_tuple(schema: &Schema, items: &[Type], reader: &str, depth: usize) -> String {
    let items = items
        .iter()
        .enumerate()
        .map(|(index, item)| render_decode_type(schema, item, reader, depth + index))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{items}]")
}

/// Render one map decoder expression.
fn render_decode_map(
    schema: &Schema,
    key: &Type,
    item: &Type,
    reader: &str,
    depth: usize,
) -> String {
    let len = format!("length{depth}");
    let out = format!("items{depth}");
    let key_value = render_decode_type(schema, key, reader, depth + 1);
    let item_value = render_decode_type(schema, item, reader, depth + 2);

    if matches!(key, Type::String) {
        format!(
            "(() => {{ const {len} = {reader}.readNumber(); const {out}: Record<string, {}> = {{}}; for (let index = 0; index < {len}; index += 1) {{ const key = {key_value}; {out}[key] = {item_value}; }} return {out}; }})()",
            render_type(schema, item)
        )
    } else {
        format!(
            "(() => {{ const {len} = {reader}.readNumber(); const {out} = new Map<{}, {}>(); for (let index = 0; index < {len}; index += 1) {{ {out}.set({key_value}, {item_value}); }} return {out}; }})()",
            render_type(schema, key),
            render_type(schema, item)
        )
    }
}

/// Return one property access key.
pub(super) fn property_key(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_digit()) {
        name.to_string()
    } else {
        format!("{name:?}")
    }
}

/// Return one generated encoder function name.
pub(super) fn encode_name(name: &str) -> String {
    format!("encode{name}")
}

/// Return one generated decoder function name.
pub(super) fn decode_name(name: &str) -> String {
    format!("decode{name}")
}
