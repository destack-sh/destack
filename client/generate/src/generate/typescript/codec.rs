use crate::generate::schema::{Field, Item, Payload, Schema, Shape, Type, Variant};

use super::item::render_type;
use super::name::{identifier, is_identifier};
use super::path::TypeNames;
use super::text::Text;

/// Render encode and decode functions for one protocol item.
pub(super) fn render_codec_item(
    schema: &Schema,
    item: &Item,
    type_names: &TypeNames,
    text: &mut Text,
) {
    text.line(format!("/** Encode one {}. */", item.name));
    text.line(format!(
        "export function {}(writer: BinaryWriter, value: {}): void {{",
        encode_name(&item.name),
        item.name
    ));
    render_encode_item(schema, item, type_names, text, "value", "    ");
    text.line("}");
    text.blank();

    text.line(format!("/** Decode one {}. */", item.name));
    text.line(format!(
        "export function {}(reader: BinaryReader): {} {{",
        decode_name(&item.name),
        item.name
    ));
    render_decode_item(schema, item, type_names, text, "    ");
    text.line("}");
    text.blank();

    text.line(format!(
        "/** Return one JSON value for one {}. */",
        item.name
    ));
    text.line(format!(
        "export function {}(value: {}): Json {{",
        to_json_name(&item.name),
        item.name
    ));
    render_to_json_item(schema, item, type_names, text, "value", "    ");
    text.line("}");
    text.blank();

    text.line(format!(
        "/** Return one {} from one JSON value. */",
        item.name
    ));
    text.line(format!(
        "export function {}(value: Json): {} {{",
        from_json_name(&item.name),
        item.name
    ));
    render_from_json_item(schema, item, type_names, text, "value", "    ");
    text.line("}");
    text.blank();
}

/// Render one item encoder body.
fn render_encode_item(
    schema: &Schema,
    item: &Item,
    type_names: &TypeNames,
    text: &mut Text,
    value: &str,
    indent: &str,
) {
    if let Some(ty) = item.scalar_newtype() {
        render_encode_type(schema, type_names, text, ty, value, indent, 0);

        return;
    }

    match &item.shape {
        Shape::Struct(fields) => {
            for (index, field) in fields.iter().enumerate() {
                let field_value = property_access(value, &field.label());
                render_encode_type(
                    schema,
                    type_names,
                    text,
                    &field.ty,
                    &field_value,
                    indent,
                    index,
                );
            }
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.key) => {
            render_encode_unit_enum(text, variants, value, indent);
        }
        Shape::Enum(variants) => {
            render_encode_payload_enum(schema, type_names, text, variants, value, indent);
        }
    }
}

/// Render one item decoder body.
fn render_decode_item(
    schema: &Schema,
    item: &Item,
    type_names: &TypeNames,
    text: &mut Text,
    indent: &str,
) {
    if let Some(ty) = item.scalar_newtype() {
        let value = render_decode_type(schema, type_names, ty, "reader", 0);

        text.line(format!("{indent}return {value};"));

        return;
    }

    match &item.shape {
        Shape::Struct(fields) => {
            for (index, field) in fields.iter().enumerate() {
                let value = render_decode_type(schema, type_names, &field.ty, "reader", index);
                let name = field_local_name(field);

                text.line(format!("{indent}const {name} = {value};"));
            }
            if !fields.is_empty() {
                text.blank();
            }

            text.line(format!("{indent}return {{"));
            for field in fields {
                render_decoded_field(text, field, indent);
            }
            text.line(format!("{indent}}};"));
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.key) => {
            render_decode_unit_enum(text, variants, indent);
        }
        Shape::Enum(variants) => {
            render_decode_payload_enum(schema, type_names, text, variants, indent);
        }
    }
}

/// Render one item JSON encoder body.
fn render_to_json_item(
    schema: &Schema,
    item: &Item,
    type_names: &TypeNames,
    text: &mut Text,
    value: &str,
    indent: &str,
) {
    if let Some(ty) = item.scalar_newtype() {
        let value = render_to_json_type(schema, type_names, ty, value, 0);

        text.line(format!("{indent}return {value};"));

        return;
    }

    match &item.shape {
        Shape::Struct(fields) => {
            text.line(format!("{indent}return {{"));
            for field in fields {
                render_json_field(schema, type_names, text, field, value, indent);
            }
            text.line(format!("{indent}}};"));
        }
        Shape::Enum(_) if schema.is_unit_enum(&item.key) => {
            text.line(format!("{indent}return {value};"));
        }
        Shape::Enum(variants) => {
            render_to_json_payload_enum(schema, type_names, text, variants, value, indent);
        }
    }
}

/// Render one item JSON decoder body.
fn render_from_json_item(
    schema: &Schema,
    item: &Item,
    type_names: &TypeNames,
    text: &mut Text,
    value: &str,
    indent: &str,
) {
    if let Some(ty) = item.scalar_newtype() {
        let value = render_from_json_type(schema, type_names, ty, value, 0);

        text.line(format!("{indent}return {value};"));

        return;
    }

    match &item.shape {
        Shape::Struct(fields) => {
            text.line(format!("{indent}const object = jsonObject({value});"));
            if !fields.is_empty() {
                text.blank();
            }

            text.line(format!("{indent}return {{"));
            for field in fields {
                render_from_json_field(schema, type_names, text, field, indent);
            }
            text.line(format!("{indent}}};"));
        }
        Shape::Enum(variants) if schema.is_unit_enum(&item.key) => {
            render_from_json_unit_enum(text, variants, value, indent);
        }
        Shape::Enum(variants) => {
            render_from_json_payload_enum(schema, type_names, text, variants, value, indent);
        }
    }
}

/// Render one JSON struct field.
fn render_json_field(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    field: &Field,
    value: &str,
    indent: &str,
) {
    let property = field.label();
    let field_value = property_access(value, &property);

    if let Type::Option(item) = &field.ty {
        let json = render_to_json_type(schema, type_names, item, &field_value, 0);
        let assignment = property_assignment(&property, &json);

        text.line(format!(
            "{indent}    ...({field_value} === undefined ? {{}} : {{ {assignment} }}),"
        ));
    } else {
        let json = render_to_json_type(schema, type_names, &field.ty, &field_value, 0);
        let assignment = property_assignment(&property, &json);

        text.line(format!("{indent}    {assignment},"));
    }
}

/// Render one JSON struct field decoder.
fn render_from_json_field(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    field: &Field,
    indent: &str,
) {
    let property = field.label();
    let json = if let Type::Option(item) = &field.ty {
        let value = render_from_json_type(schema, type_names, item, "value", 0);

        format!("jsonOptional(object, {:?}, (value) => {value})", property)
    } else {
        let field_value = format!("jsonField(object, {property:?})");

        render_from_json_type(schema, type_names, &field.ty, &field_value, 0)
    };
    let assignment = property_assignment(&property, &json);

    text.line(format!("{indent}    {assignment},"));
}

/// Render one payload enum JSON encoder.
fn render_to_json_payload_enum(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    variants: &[Variant],
    value: &str,
    indent: &str,
) {
    text.line(format!("{indent}switch ({value}.kind) {{"));
    for variant in variants {
        text.line(format!("{indent}    case {:?}:", variant.label()));
        text.line(format!("{indent}        return {{"));
        text.line(format!("{indent}            kind: {:?},", variant.label()));

        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(ty) => {
                let property = variant.payload_field_name();
                let field_value = payload_property_access(value, &property);
                let json = render_to_json_type(schema, type_names, ty, &field_value, 0);
                let assignment = property_assignment(&property, &json);

                text.line(format!("{indent}            {assignment},"));
            }
            Payload::Struct(fields) => {
                for field in fields {
                    render_json_payload_field(schema, type_names, text, field, value, indent);
                }
            }
        }

        text.line(format!("{indent}        }};"));
    }
    text.line(format!("{indent}}}"));
    text.blank();
    text.line(format!(
        "{indent}throw new SerdeError(\"unknown enum variant\");"
    ));
}

/// Render one JSON payload enum field.
fn render_json_payload_field(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    field: &Field,
    value: &str,
    indent: &str,
) {
    let property = payload_property_name(&field.label());
    let field_value = property_access(value, &property);

    if let Type::Option(item) = &field.ty {
        let json = render_to_json_type(schema, type_names, item, &field_value, 0);
        let assignment = property_assignment(&property, &json);

        text.line(format!(
            "{indent}            ...({field_value} === undefined ? {{}} : {{ {assignment} }}),"
        ));
    } else {
        let json = render_to_json_type(schema, type_names, &field.ty, &field_value, 0);
        let assignment = property_assignment(&property, &json);

        text.line(format!("{indent}            {assignment},"));
    }
}

/// Render one unit enum JSON decoder.
fn render_from_json_unit_enum(text: &mut Text, variants: &[Variant], value: &str, indent: &str) {
    text.line(format!("{indent}const variant = jsonString({value});"));
    text.blank();
    text.line(format!("{indent}switch (variant) {{"));
    for variant in variants {
        text.line(format!("{indent}    case {:?}:", variant.label()));
        text.line(format!("{indent}        return {:?};", variant.label()));
    }
    text.line(format!("{indent}}}"));
    text.blank();
    text.line(format!(
        "{indent}throw new SerdeError(`unknown enum variant: ${{variant}}`);"
    ));
}

/// Render one payload enum JSON decoder.
fn render_from_json_payload_enum(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    variants: &[Variant],
    value: &str,
    indent: &str,
) {
    text.line(format!("{indent}const object = jsonObject({value});"));
    text.line(format!(
        "{indent}const kind = jsonString(jsonField(object, \"kind\"));"
    ));
    text.blank();
    text.line(format!("{indent}switch (kind) {{"));
    for variant in variants {
        text.line(format!("{indent}    case {:?}:", variant.label()));
        text.line(format!("{indent}        return {{"));
        text.line(format!("{indent}            kind,"));

        match &variant.payload {
            Payload::Unit => {}
            Payload::Tuple(ty) => {
                let property = variant.payload_field_name();
                let field_value = format!("jsonField(object, {property:?})");
                let json = render_from_json_type(schema, type_names, ty, &field_value, 0);
                let assignment = property_assignment(&property, &json);

                text.line(format!("{indent}            {assignment},"));
            }
            Payload::Struct(fields) => {
                for field in fields {
                    render_from_json_payload_field(schema, type_names, text, field, indent);
                }
            }
        }

        text.line(format!("{indent}        }};"));
    }
    text.line(format!("{indent}}}"));
    text.blank();
    text.line(format!(
        "{indent}throw new SerdeError(`unknown enum variant: ${{kind}}`);"
    ));
}

/// Render one JSON payload enum field decoder.
fn render_from_json_payload_field(
    schema: &Schema,
    type_names: &TypeNames,
    text: &mut Text,
    field: &Field,
    indent: &str,
) {
    let property = payload_property_name(&field.label());
    let json = if let Type::Option(item) = &field.ty {
        let value = render_from_json_type(schema, type_names, item, "value", 0);

        format!("jsonOptional(object, {:?}, (value) => {value})", property)
    } else {
        let field_value = format!("jsonField(object, {property:?})");

        render_from_json_type(schema, type_names, &field.ty, &field_value, 0)
    };
    let assignment = property_assignment(&property, &json);

    text.line(format!("{indent}            {assignment},"));
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
    type_names: &TypeNames,
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
                let field = payload_property_access(value, &variant.payload_field_name());
                render_encode_type(
                    schema,
                    type_names,
                    text,
                    ty,
                    &field,
                    &format!("{indent}        "),
                    0,
                );
            }
            Payload::Struct(fields) => {
                for (field_index, field) in fields.iter().enumerate() {
                    let field_value = payload_property_access(value, &field.label());
                    render_encode_type(
                        schema,
                        type_names,
                        text,
                        &field.ty,
                        &field_value,
                        &format!("{indent}        "),
                        field_index,
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
    type_names: &TypeNames,
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
                let value = render_decode_type(schema, type_names, ty, "reader", 0);
                let field = payload_property_name(&variant.payload_field_name());
                let name = identifier(&field);
                text.line(format!("{indent}        const {name} = {value};"));
                text.blank();
                text.line(format!(
                    "{indent}        return {{ kind: {:?}, {} }};",
                    variant.label(),
                    property_assignment(&field, &name)
                ));
            }
            Payload::Struct(fields) => {
                for (field_index, field) in fields.iter().enumerate() {
                    let value =
                        render_decode_type(schema, type_names, &field.ty, "reader", field_index);
                    let name = payload_field_local_name(field);

                    text.line(format!("{indent}        const {name} = {value};"));
                }
                if !fields.is_empty() {
                    text.blank();
                }
                text.line(format!("{indent}        return {{"));
                text.line(format!("{indent}            kind: {:?},", variant.label()));
                for field in fields {
                    render_decoded_payload_field(text, field, &format!("{indent}        "));
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
fn render_decoded_field(text: &mut Text, field: &Field, indent: &str) {
    let property = field.label();
    let name = identifier(&property);
    let assignment = property_assignment(&property, &name);

    if matches!(field.ty, Type::Option(_)) {
        text.line(format!(
            "{indent}    ...({name} === undefined ? {{}} : {{ {assignment} }}),"
        ));
    } else {
        text.line(format!("{indent}    {assignment},"));
    }
}

/// Render one decoded payload enum field.
fn render_decoded_payload_field(text: &mut Text, field: &Field, indent: &str) {
    let property = payload_property_name(&field.label());
    let name = identifier(&property);
    let assignment = property_assignment(&property, &name);

    if matches!(field.ty, Type::Option(_)) {
        text.line(format!(
            "{indent}    ...({name} === undefined ? {{}} : {{ {assignment} }}),"
        ));
    } else {
        text.line(format!("{indent}    {assignment},"));
    }
}

/// Return one decoded struct field local name.
fn field_local_name(field: &Field) -> String {
    identifier(&field.label())
}

/// Return one decoded payload enum field local name.
fn payload_field_local_name(field: &Field) -> String {
    identifier(&payload_property_name(&field.label()))
}

/// Return one object property assignment.
fn property_assignment(property: &str, value: &str) -> String {
    let key = property_key(property);
    if key == value {
        value.to_string()
    } else {
        format!("{key}: {value}")
    }
}

/// Render one value encoder.
fn render_encode_type(
    schema: &Schema,
    type_names: &TypeNames,
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
        Type::Float(_) => ty.unsupported_client_type(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            text.line(format!("{indent}writer.writeByteSlice({value});"));
        }
        Type::Vec(item) => {
            render_encode_sequence(schema, type_names, text, item, value, indent, depth)
        }
        Type::Option(item) => {
            text.line(format!(
                "{indent}writer.writeOption({value}, (value{depth}) => {{"
            ));
            render_encode_type(
                schema,
                type_names,
                text,
                item,
                &format!("value{depth}"),
                &format!("{indent}    "),
                depth + 1,
            );
            text.line(format!("{indent}}});"));
        }
        Type::Array(item, _) => {
            render_encode_array(schema, type_names, text, item, value, indent, depth)
        }
        Type::Tuple(items) => {
            render_encode_tuple(schema, type_names, text, items, value, indent, depth)
        }
        Type::Map(key, item) => {
            render_encode_map(schema, type_names, text, key, item, value, indent, depth)
        }
        Type::Json => text.line(format!("{indent}writer.writeJson({value});")),
        Type::Named { key, name } => {
            let encoder = type_names.encoder(key, name);
            text.line(format!("{indent}{encoder}(writer, {value});"))
        }
    }
}

/// Render one sequence encoder.
fn render_encode_sequence(
    schema: &Schema,
    type_names: &TypeNames,
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
        type_names,
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
    type_names: &TypeNames,
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
        type_names,
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
    type_names: &TypeNames,
    text: &mut Text,
    items: &[Type],
    value: &str,
    indent: &str,
    depth: usize,
) {
    for (index, item) in items.iter().enumerate() {
        render_encode_type(
            schema,
            type_names,
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
    type_names: &TypeNames,
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
        type_names,
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
        type_names,
        text,
        key,
        &format!("{entry}.{key_name}"),
        &format!("{indent}    "),
        depth + 1,
    );
    render_encode_type(
        schema,
        type_names,
        text,
        item,
        &format!("{entry}.{item_name}"),
        &format!("{indent}    "),
        depth + 1,
    );
    text.line(format!("{indent}}}"));
}

/// Render one value decoder expression.
fn render_decode_type(
    schema: &Schema,
    type_names: &TypeNames,
    ty: &Type,
    reader: &str,
    depth: usize,
) -> String {
    match ty {
        Type::String => format!("{reader}.readString()"),
        Type::Bool => format!("{reader}.readBool()"),
        Type::Char => format!("{reader}.readChar()"),
        Type::U8 => format!("{reader}.readByte()"),
        Type::U32 | Type::Usize => format!("{reader}.readNumber()"),
        Type::U64 | Type::U128 => format!("{reader}.readUnsigned()"),
        Type::Signed(8) => format!("{reader}.readI8()"),
        Type::Signed(16 | 32) => format!("{reader}.readSignedNumber()"),
        Type::Signed(64 | 128) => format!("{reader}.readSigned()"),
        Type::Signed(_) => ty.unsupported_client_type(),
        Type::Float(32) => format!("{reader}.readF32()"),
        Type::Float(64) => format!("{reader}.readF64()"),
        Type::Float(_) => ty.unsupported_client_type(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => format!("{reader}.readByteSlice()"),
        Type::Vec(item) => render_decode_sequence(schema, type_names, item, reader, depth),
        Type::Option(item) => {
            let value = render_decode_type(schema, type_names, item, reader, depth + 1);

            format!("{reader}.readOption(() => {value})")
        }
        Type::Array(item, len) => {
            render_decode_array(schema, type_names, item, *len, reader, depth)
        }
        Type::Tuple(items) => render_decode_tuple(schema, type_names, items, reader, depth),
        Type::Map(key, item) => render_decode_map(schema, type_names, key, item, reader, depth),
        Type::Json => format!("{reader}.readJson()"),
        Type::Named { key, name } => {
            let decoder = type_names.decoder(key, name);

            format!("{decoder}({reader})")
        }
    }
}

/// Render one sequence decoder expression.
fn render_decode_sequence(
    schema: &Schema,
    type_names: &TypeNames,
    item: &Type,
    reader: &str,
    depth: usize,
) -> String {
    let len = format!("length{depth}");
    let out = format!("items{depth}");
    let value = render_decode_type(schema, type_names, item, reader, depth + 1);

    let item = render_type(schema, type_names, item);

    format!(
        "(() => {{ const {len} = {reader}.readNumber(); const {out}: Array<{item}> = []; for (let index = 0; index < {len}; index += 1) {{ {out}.push({value}); }} return {out}; }})()"
    )
}

/// Render one fixed array decoder expression.
fn render_decode_array(
    schema: &Schema,
    type_names: &TypeNames,
    item: &Type,
    len: usize,
    reader: &str,
    depth: usize,
) -> String {
    if matches!(item, Type::U8) {
        return format!("{reader}.readBytes({len})");
    }

    let items = (0..len)
        .map(|index| render_decode_type(schema, type_names, item, reader, depth + index))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{items}] as const")
}

/// Render one tuple decoder expression.
fn render_decode_tuple(
    schema: &Schema,
    type_names: &TypeNames,
    items: &[Type],
    reader: &str,
    depth: usize,
) -> String {
    let items = items
        .iter()
        .enumerate()
        .map(|(index, item)| render_decode_type(schema, type_names, item, reader, depth + index))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{items}] as const")
}

/// Render one map decoder expression.
fn render_decode_map(
    schema: &Schema,
    type_names: &TypeNames,
    key: &Type,
    item: &Type,
    reader: &str,
    depth: usize,
) -> String {
    let len = format!("length{depth}");
    let out = format!("items{depth}");
    let key_value = render_decode_type(schema, type_names, key, reader, depth + 1);
    let item_value = render_decode_type(schema, type_names, item, reader, depth + 2);

    if matches!(key, Type::String) {
        format!(
            "(() => {{ const {len} = {reader}.readNumber(); const {out}: Record<string, {}> = {{}}; for (let index = 0; index < {len}; index += 1) {{ const key = {key_value}; {out}[key] = {item_value}; }} return {out}; }})()",
            render_type(schema, type_names, item)
        )
    } else {
        format!(
            "(() => {{ const {len} = {reader}.readNumber(); const {out} = new Map<{}, {}>(); for (let index = 0; index < {len}; index += 1) {{ {out}.set({key_value}, {item_value}); }} return {out}; }})()",
            render_type(schema, type_names, key),
            render_type(schema, type_names, item)
        )
    }
}

/// Render one JSON encoder expression.
fn render_to_json_type(
    schema: &Schema,
    type_names: &TypeNames,
    ty: &Type,
    value: &str,
    depth: usize,
) -> String {
    match ty {
        Type::String
        | Type::Bool
        | Type::U8
        | Type::U32
        | Type::Usize
        | Type::Signed(8 | 16 | 32)
        | Type::Float(_) => value.to_string(),
        Type::Json => format!("{value} as Json"),
        Type::Char => value.to_string(),
        Type::U64 | Type::U128 | Type::Signed(64 | 128) => format!("{value}.toString()"),
        Type::Signed(_) => ty.unsupported_client_type(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            format!("bytesToJson({value})")
        }
        Type::Vec(item) => render_to_json_sequence(schema, type_names, item, value, depth),
        Type::Option(item) => {
            let item = render_to_json_type(schema, type_names, item, value, depth);

            format!("{value} === undefined ? null : {item}")
        }
        Type::Array(item, _) if matches!(item.as_ref(), Type::U8) => {
            format!("bytesToJson({value})")
        }
        Type::Array(item, _) => render_to_json_sequence(schema, type_names, item, value, depth),
        Type::Tuple(items) => render_to_json_tuple(schema, type_names, items, value, depth),
        Type::Map(key, item) => render_to_json_map(schema, type_names, key, item, value, depth),
        Type::Named { key, name } => {
            let encoder = type_names.json_encoder(key, name);

            format!("{encoder}({value})")
        }
    }
}

/// Render one JSON decoder expression.
fn render_from_json_type(
    schema: &Schema,
    type_names: &TypeNames,
    ty: &Type,
    value: &str,
    depth: usize,
) -> String {
    match ty {
        Type::String | Type::Char => format!("jsonString({value})"),
        Type::Bool => format!("jsonBool({value})"),
        Type::U8 | Type::U32 | Type::Usize | Type::Signed(8 | 16 | 32) => {
            format!("jsonInteger({value})")
        }
        Type::U64 | Type::U128 | Type::Signed(64 | 128) => format!("jsonBigint({value})"),
        Type::Signed(_) => ty.unsupported_client_type(),
        Type::Float(_) => format!("jsonNumber({value})"),
        Type::Json => value.to_string(),
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            format!("bytesFromJson({value})")
        }
        Type::Vec(item) => render_from_json_sequence(schema, type_names, item, value, depth),
        Type::Option(item) => {
            let item = render_from_json_type(schema, type_names, item, value, depth);

            format!("{value} === null ? undefined : {item}")
        }
        Type::Array(item, _) if matches!(item.as_ref(), Type::U8) => {
            format!("bytesFromJson({value})")
        }
        Type::Array(item, len) => {
            render_from_json_array(schema, type_names, item, *len, value, depth)
        }
        Type::Tuple(items) => render_from_json_tuple(schema, type_names, items, value, depth),
        Type::Map(key, item) => render_from_json_map(schema, type_names, key, item, value, depth),
        Type::Named { key, name } => {
            let decoder = type_names.json_decoder(key, name);

            format!("{decoder}({value})")
        }
    }
}

/// Render one JSON sequence encoder expression.
fn render_to_json_sequence(
    schema: &Schema,
    type_names: &TypeNames,
    item: &Type,
    value: &str,
    depth: usize,
) -> String {
    let item_name = format!("item{depth}");
    let item_value = render_to_json_type(schema, type_names, item, &item_name, depth + 1);

    format!("{value}.map(({item_name}) => {item_value})")
}

/// Render one JSON tuple encoder expression.
fn render_to_json_tuple(
    schema: &Schema,
    type_names: &TypeNames,
    items: &[Type],
    value: &str,
    depth: usize,
) -> String {
    let items = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            render_to_json_type(
                schema,
                type_names,
                item,
                &format!("{value}[{index}]"),
                depth + index,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{items}]")
}

/// Render one JSON map encoder expression.
fn render_to_json_map(
    schema: &Schema,
    type_names: &TypeNames,
    key: &Type,
    item: &Type,
    value: &str,
    depth: usize,
) -> String {
    let key_name = format!("key{depth}");
    let item_name = format!("item{depth}");
    let item_value = render_to_json_type(schema, type_names, item, &item_name, depth + 1);

    if matches!(key, Type::String) {
        format!(
            "Object.fromEntries(Object.entries({value}).map(([{key_name}, {item_name}]) => [{key_name}, {item_value}] as const))"
        )
    } else {
        let key_value = render_to_json_type(schema, type_names, key, &key_name, depth + 1);

        format!(
            "Array.from({value}.entries()).map(([{key_name}, {item_name}]) => [{key_value}, {item_value}] as const)"
        )
    }
}

/// Render one JSON sequence decoder expression.
fn render_from_json_sequence(
    schema: &Schema,
    type_names: &TypeNames,
    item: &Type,
    value: &str,
    depth: usize,
) -> String {
    let item_name = format!("item{depth}");
    let item_value = render_from_json_type(schema, type_names, item, &item_name, depth + 1);

    format!("jsonArray({value}).map(({item_name}) => {item_value})")
}

/// Render one JSON fixed array decoder expression.
fn render_from_json_array(
    schema: &Schema,
    type_names: &TypeNames,
    item: &Type,
    len: usize,
    value: &str,
    depth: usize,
) -> String {
    let values = (0..len)
        .map(|index| {
            let value = format!("items[{index}]");

            render_from_json_type(schema, type_names, item, &value, depth + index)
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "(() => {{ const items = jsonArray({value}); if (items.length !== {len}) {{ throw new SerdeError(`expected JSON array length {len}: ${{items.length}}`); }} return [{values}] as const; }})()"
    )
}

/// Render one JSON tuple decoder expression.
fn render_from_json_tuple(
    schema: &Schema,
    type_names: &TypeNames,
    items: &[Type],
    value: &str,
    depth: usize,
) -> String {
    let values = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let value = format!("items[{index}]");

            render_from_json_type(schema, type_names, item, &value, depth + index)
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "(() => {{ const items = jsonArray({value}); if (items.length !== {}) {{ throw new SerdeError(`expected JSON tuple length {}: ${{items.length}}`); }} return [{values}] as const; }})()",
        items.len(),
        items.len(),
    )
}

/// Render one JSON map decoder expression.
fn render_from_json_map(
    schema: &Schema,
    type_names: &TypeNames,
    key: &Type,
    item: &Type,
    value: &str,
    depth: usize,
) -> String {
    let key_name = format!("key{depth}");
    let item_name = format!("item{depth}");
    let item_value = render_from_json_type(schema, type_names, item, &item_name, depth + 1);

    if matches!(key, Type::String) {
        format!(
            "Object.fromEntries(Object.entries(jsonObject({value})).map(([{key_name}, {item_name}]) => [{key_name}, {item_value}] as const))"
        )
    } else {
        let key_value = render_from_json_type(schema, type_names, key, &key_name, depth + 1);

        format!(
            "new Map(jsonArray({value}).map((entry) => {{ const items = jsonArray(entry); if (items.length !== 2) {{ throw new SerdeError(`expected JSON map entry length 2: ${{items.length}}`); }} const {key_name} = items[0]; const {item_name} = items[1]; return [{key_value}, {item_value}] as const; }}))"
        )
    }
}

/// Return one property access key.
pub(super) fn property_key(name: &str) -> String {
    if is_identifier(name) || name.chars().all(|c| c.is_ascii_digit()) {
        name.to_string()
    } else {
        format!("{name:?}")
    }
}

/// Return one property access expression.
pub(super) fn property_access(value: &str, name: &str) -> String {
    if is_identifier(name) {
        format!("{value}.{name}")
    } else {
        format!("{value}[{}]", property_key(name))
    }
}

/// Return one property key for TypeScript indexed access types.
pub(super) fn type_property_key(name: &str) -> String {
    format!("{name:?}")
}

/// Return one payload enum property name.
pub(super) fn payload_property_name(name: &str) -> String {
    if name == "kind" {
        "kindValue".to_string()
    } else {
        name.to_string()
    }
}

/// Return one payload enum property access expression.
pub(super) fn payload_property_access(value: &str, name: &str) -> String {
    property_access(value, &payload_property_name(name))
}

/// Return one generated encoder function name.
pub(super) fn encode_name(name: &str) -> String {
    format!("encode{name}")
}

/// Return one generated decoder function name.
pub(super) fn decode_name(name: &str) -> String {
    format!("decode{name}")
}

/// Return one generated JSON encoder function name.
pub(super) fn to_json_name(name: &str) -> String {
    format!("toJson{name}")
}

/// Return one generated JSON decoder function name.
pub(super) fn from_json_name(name: &str) -> String {
    format!("fromJson{name}")
}
