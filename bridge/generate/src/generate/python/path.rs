use std::collections::{BTreeMap, BTreeSet};

use crate::generate::schema::{ModulePath, Schema, SchemaModule, Shape};

use super::codec::{decode_name, encode_name};
use super::item::protocol_variant_name;
use super::text::Text;

/// Render Python imports referenced by one generated protocol module.
pub(super) fn render_protocol_imports(schema: &Schema, module: &SchemaModule) -> String {
    let mut imports = BTreeMap::<String, BTreeSet<String>>::new();
    let mut runtime_imports = BTreeSet::<String>::new();

    for item in schema.referenced_types(&module.names) {
        let path = schema.module_path(&item.name);
        if path == &module.path {
            continue;
        }

        let module_path = protocol_absolute_module_path(schema, path);
        imports
            .entry(module_path.clone())
            .or_default()
            .insert(item.name.clone());
        runtime_imports.insert(module_path);
    }

    let mut text = Text::new();
    for path in &runtime_imports {
        text.line(format!("import {path}"));
    }

    if !runtime_imports.is_empty() {
        text.blank();
    }

    if !imports.is_empty() {
        text.line("if TYPE_CHECKING:");
    }

    for (path, names) in imports {
        text.line(format!("    from {path} import ("));

        for name in names {
            text.line(format!("        {name},"));
        }

        text.line("    )");
        text.blank();
    }

    text.finish()
}

/// Return all Python names emitted by one protocol module.
pub(super) fn protocol_module_names(schema: &Schema, module: &SchemaModule) -> Vec<String> {
    let mut names = Vec::new();

    for name in &module.names {
        let item = schema.item(name);
        names.push(name.clone());
        names.push(encode_name(name));
        names.push(decode_name(name));

        if let Shape::Enum(variants) = &item.shape
            && !schema.is_unit_enum(&item.name)
        {
            names.extend(
                variants
                    .iter()
                    .map(|variant| protocol_variant_name(item, variant)),
            );
        }
    }

    names
}

/// Return generated Python protocol module path segments.
pub(super) fn protocol_python_segments(schema: &Schema, module: &SchemaModule) -> Vec<String> {
    let mut segments = module.path.segments().to_vec();
    if protocol_module_path_collides(schema, module.path.segments()) {
        segments.push("model".to_string());
    }

    segments
}

/// Return whether one generated Python protocol module path is also a package path.
pub(super) fn protocol_module_path_collides(schema: &Schema, segments: &[String]) -> bool {
    schema.modules.iter().any(|module| {
        let other = module.path.segments();

        other.len() > segments.len() && other.starts_with(segments)
    })
}

/// Return one absolute Python module path for a protocol schema path.
pub(super) fn protocol_absolute_module_path(schema: &Schema, path: &ModulePath) -> String {
    let mut segments = path.segments().to_vec();
    if protocol_module_path_collides(schema, path.segments()) {
        segments.push("model".to_string());
    }

    format!("destack._generated.{}", segments.join("."))
}
