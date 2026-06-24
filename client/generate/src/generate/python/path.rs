use std::collections::BTreeSet;

use crate::generate::schema::{ModulePath, Schema, SchemaModule, Shape};

use super::codec::{decode_name, encode_name, from_json_name, to_json_name};
use super::item::protocol_variant_name;
use super::name::python_segment_name;
use super::text::Text;

/// Render Python imports referenced by one generated module.
pub(super) fn render_imports(schema: &Schema, module: &SchemaModule) -> String {
    let mut runtime_imports = BTreeSet::<String>::new();

    for item in schema.referenced_types(&module.keys) {
        let path = schema.module_path(&item.key);
        if path == &module.path {
            continue;
        }

        let module_path = absolute_module_path(schema, path);
        runtime_imports.insert(module_path);
    }

    let mut text = Text::new();
    for path in &runtime_imports {
        text.line(format!("import {path}"));
    }

    if !runtime_imports.is_empty() {
        text.blank();
    }

    text.finish()
}

/// Return all Python names emitted by one module.
pub(super) fn module_names(schema: &Schema, module: &SchemaModule) -> Vec<String> {
    let mut names = Vec::new();

    for key in &module.keys {
        let item = schema.item(key);
        names.push(item.name.clone());
        names.push(encode_name(&item.name));
        names.push(decode_name(&item.name));
        names.push(to_json_name(&item.name));
        names.push(from_json_name(&item.name));

        if let Shape::Enum(variants) = &item.shape
            && !schema.is_unit_enum(&item.key)
        {
            names.extend(
                variants
                    .iter()
                    .map(|variant| protocol_variant_name(schema, item, variant)),
            );
        }
    }

    names
}

/// Return generated Python module path segments.
pub(super) fn python_segments(schema: &Schema, path: &ModulePath) -> Vec<String> {
    let mut segments = path
        .segments()
        .iter()
        .map(|segment| python_segment_name(segment))
        .collect::<Vec<_>>();
    if module_path_collides(schema, path.segments()) {
        segments.push("model".to_string());
    }

    segments
}

/// Return whether one generated Python module path is also a package path.
pub(super) fn module_path_collides(schema: &Schema, segments: &[String]) -> bool {
    schema.modules.iter().any(|module| {
        let other = module.path.segments();

        other.len() > segments.len() && other.starts_with(segments)
    })
}

/// Return one absolute Python module path for a schema path.
pub(super) fn absolute_module_path(schema: &Schema, path: &ModulePath) -> String {
    let segments = python_segments(schema, path);

    format!("destack._generated.{}", segments.join("."))
}
