use std::collections::BTreeSet;

use crate::generate::implementation::implementation_owners;
use crate::generate::schema::{Item, Payload, Schema, SchemaModule, Shape, Type, Variant};

use super::codec::{render_codec_item, render_codec_stub_item};
use super::item::render_item;
use super::package::render_all;
use super::path::{module_names, render_imports};
use super::text::Text;

const SERDE_IMPORT_ORDER: &[&str] = &[
    "BinaryReader",
    "BinaryWriter",
    "Json",
    "SerdeError",
    "bytes_from_json",
    "bytes_to_json",
    "json_array",
    "json_array_length",
    "json_bool",
    "json_field",
    "json_int",
    "json_number",
    "json_object",
    "json_optional",
    "json_string",
    "nested_bytes",
];

pub(super) fn render_module(schema: &Schema, module: &SchemaModule) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.raw(render_runtime_imports(schema, module, false));
    text.raw(render_implementation_imports(schema, module));
    text.raw(render_imports(schema, module));

    for name in &module.keys {
        let item = schema.item(name);
        text.raw(render_item(schema, module, item, false));
        text.raw(render_codec_item(schema, module, item));
    }

    render_all(&mut text, &module_names(schema, module));

    text.finish()
}

/// Render one generated Python protocol module stub.
pub(super) fn render_module_stub(schema: &Schema, module: &SchemaModule) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.raw(render_runtime_imports(schema, module, true));
    text.raw(render_implementation_imports(schema, module));
    text.raw(render_imports(schema, module));

    for name in &module.keys {
        let item = schema.item(name);
        text.raw(render_item(schema, module, item, true));
        text.raw(render_codec_stub_item(item));
    }

    render_all(&mut text, &module_names(schema, module));

    text.finish()
}

/// Render Python implementation imports for one generated module.
fn render_implementation_imports(schema: &Schema, module: &SchemaModule) -> String {
    let mut text = Text::new();

    let owners = implementation_owners(module)
        .into_iter()
        .filter(|owner| {
            !implementation_item(schema, module, owner)
                .scalar_newtype()
                .is_some()
        })
        .collect::<Vec<_>>();

    for owner in &owners {
        let path = python_implementation_import(owner.module());
        text.line(format!("from {path} import ("));
        text.line(format!("    {},", owner.python_impl()));
        text.line(")");
    }

    if !owners.is_empty() {
        text.blank();
    }

    text.finish()
}

/// Return one absolute Python implementation import path.
fn python_implementation_import(implementation_module: &[&str]) -> String {
    format!("destack._impl.{}", implementation_module.join("."))
}

/// Return the generated item matched by one implementation owner.
fn implementation_item<'schema>(
    schema: &'schema Schema,
    module: &SchemaModule,
    owner: &crate::generate::implementation::ImplementationOwner,
) -> &'schema Item {
    module
        .keys
        .iter()
        .map(|key| schema.item(key))
        .find(|item| owner.matches(item))
        .expect("implementation owner should match one module item")
}

/// Render Python runtime imports needed by one generated module.
fn render_runtime_imports(schema: &Schema, module: &SchemaModule, is_stub: bool) -> String {
    let items = module
        .keys
        .iter()
        .map(|key| schema.item(key))
        .collect::<Vec<_>>();
    let mut text = Text::new();

    if items.iter().any(|item| item_needs_builtins(item)) {
        text.line("import builtins");
        text.blank();
    }

    let mut collections = Vec::new();
    if items.iter().any(|item| item.has_map()) {
        collections.push("Mapping");
    }
    if items.iter().any(|item| item_needs_sequence(item)) {
        collections.push("Sequence");
    }
    if !collections.is_empty() {
        text.line(format!(
            "from collections.abc import {}",
            collections.join(", ")
        ));
    }

    if items.iter().any(|item| item_needs_dataclass(schema, item)) {
        text.line("from dataclasses import dataclass");
    }

    if items
        .iter()
        .any(|item| item.scalar_newtype().is_some() || item_needs_json(item) || item.is_enum())
    {
        text.line("import typing");
    }
    let needs_typing_import = !collections.is_empty()
        || items.iter().any(|item| item_needs_dataclass(schema, item))
        || items
            .iter()
            .any(|item| item.scalar_newtype().is_some() || item_needs_json(item) || item.is_enum());
    if needs_typing_import {
        text.blank();
    }

    let serde = serde_imports(&items, is_stub);
    text.line(format!(
        "from destack.protocol.serde import {}",
        serde.join(", ")
    ));
    text.blank();

    text.finish()
}

/// Return serde runtime imports needed by one generated Python module.
fn serde_imports(items: &[&Item], is_stub: bool) -> Vec<&'static str> {
    let mut imports = BTreeSet::new();
    imports.insert("BinaryReader");
    imports.insert("BinaryWriter");
    imports.insert("Json");

    if !is_stub {
        for item in items {
            collect_item_imports(&mut imports, item);
        }
    }

    SERDE_IMPORT_ORDER
        .iter()
        .copied()
        .filter(|name| imports.contains(name))
        .collect()
}

/// Collect serde runtime imports for one schema item.
fn collect_item_imports(imports: &mut BTreeSet<&'static str>, item: &Item) {
    if let Some(ty) = item.scalar_newtype() {
        collect_type_imports(imports, ty);

        return;
    }

    match &item.shape {
        Shape::Struct(fields) => {
            imports.insert("json_object");
            if !fields.is_empty() {
                imports.insert("json_field");
            }

            for field in fields {
                collect_type_imports(imports, &field.ty);
            }
        }
        Shape::Enum(variants) => {
            imports.insert("SerdeError");

            for variant in variants {
                collect_payload_imports(imports, &variant.payload);
            }

            if variants
                .iter()
                .all(|variant| matches!(variant.payload, Payload::Unit))
            {
                imports.insert("json_string");
            } else {
                imports.insert("json_field");
                imports.insert("json_object");
                imports.insert("json_string");
            }
        }
    }
}

/// Collect serde runtime imports for one enum payload.
fn collect_payload_imports(imports: &mut BTreeSet<&'static str>, payload: &Payload) {
    match payload {
        Payload::Unit => {}
        Payload::Tuple(ty) => collect_type_imports(imports, ty),
        Payload::Struct(fields) => {
            for field in fields {
                collect_type_imports(imports, &field.ty);
            }
        }
    }
}

/// Collect serde runtime imports for one type reference.
fn collect_type_imports(imports: &mut BTreeSet<&'static str>, ty: &Type) {
    match ty {
        Type::String | Type::Char => {
            imports.insert("json_string");
        }
        Type::Bool => {
            imports.insert("json_bool");
        }
        Type::U8 | Type::U32 | Type::U64 | Type::U128 | Type::Signed(_) | Type::Usize => {
            imports.insert("json_int");
        }
        Type::Float(_) => {
            imports.insert("json_number");
        }
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            imports.insert("bytes_from_json");
            imports.insert("bytes_to_json");
        }
        Type::Vec(item) => {
            imports.insert("json_array");
            collect_type_imports(imports, item);
        }
        Type::Option(item) => {
            imports.insert("json_optional");
            collect_type_imports(imports, item);
        }
        Type::Array(item, _) if matches!(item.as_ref(), Type::U8) => {
            imports.insert("bytes_from_json");
            imports.insert("bytes_to_json");
        }
        Type::Array(item, _) => {
            imports.insert("json_array");
            imports.insert("json_array_length");
            collect_type_imports(imports, item);
        }
        Type::Tuple(items) => {
            imports.insert("json_array");
            imports.insert("json_array_length");

            for item in items {
                collect_type_imports(imports, item);
            }
        }
        Type::Map(key, item) => {
            imports.insert("json_array");
            imports.insert("json_object");
            imports.insert("nested_bytes");
            collect_type_imports(imports, key);
            collect_type_imports(imports, item);
        }
        Type::Json | Type::Named { .. } => {}
    }
}

/// Return whether this item needs a dataclass decorator.
fn item_needs_dataclass(schema: &Schema, item: &Item) -> bool {
    if item.scalar_newtype().is_some() {
        return false;
    }

    match &item.shape {
        Shape::Struct(_) => true,
        Shape::Enum(_) => !schema.is_unit_enum(&item.key),
    }
}

/// Return whether this item uses the Python builtins module.
fn item_needs_builtins(item: &Item) -> bool {
    item_types(item).iter().any(|ty| type_needs_builtins(ty))
}

/// Return whether this item uses the Python Sequence type.
fn item_needs_sequence(item: &Item) -> bool {
    item_types(item).iter().any(|ty| type_needs_sequence(ty))
}

/// Return whether this item uses the Python Any type.
fn item_needs_json(item: &Item) -> bool {
    item_types(item).iter().any(|ty| type_needs_json(ty))
}

/// Return direct field and payload types for one item.
fn item_types(item: &Item) -> Vec<&Type> {
    if let Some(ty) = item.scalar_newtype() {
        return vec![ty];
    }

    match &item.shape {
        Shape::Struct(fields) => fields.iter().map(|field| &field.ty).collect(),
        Shape::Enum(variants) => variants.iter().flat_map(variant_types).collect(),
    }
}

/// Return direct payload types for one variant.
fn variant_types(variant: &Variant) -> Vec<&Type> {
    match &variant.payload {
        Payload::Unit => Vec::new(),
        Payload::Tuple(ty) => vec![ty],
        Payload::Struct(fields) => fields.iter().map(|field| &field.ty).collect(),
    }
}

/// Return whether this type uses the Python builtins module.
fn type_needs_builtins(ty: &Type) -> bool {
    match ty {
        Type::Vec(item) | Type::Array(item, _) if matches!(item.as_ref(), Type::U8) => true,
        Type::Vec(item) | Type::Option(item) | Type::Array(item, _) => type_needs_builtins(item),
        Type::Tuple(types) => types.iter().any(type_needs_builtins),
        Type::Map(key, value) => type_needs_builtins(key) || type_needs_builtins(value),
        _ => false,
    }
}

/// Return whether this type uses the Python Sequence type.
fn type_needs_sequence(ty: &Type) -> bool {
    match ty {
        Type::Vec(_) => true,
        Type::Array(item, _) if matches!(item.as_ref(), Type::U8) => true,
        Type::Option(item) | Type::Array(item, _) => type_needs_sequence(item),
        Type::Tuple(types) => types.iter().any(type_needs_sequence),
        Type::Map(key, value) => type_needs_sequence(key) || type_needs_sequence(value),
        _ => false,
    }
}

/// Return whether this type uses the Python Any type.
fn type_needs_json(ty: &Type) -> bool {
    match ty {
        Type::Json => true,
        Type::Vec(item) | Type::Option(item) | Type::Array(item, _) => type_needs_json(item),
        Type::Tuple(types) => types.iter().any(type_needs_json),
        Type::Map(key, value) => type_needs_json(key) || type_needs_json(value),
        _ => false,
    }
}
