use crate::generate::schema::{ModulePath, Schema, SchemaModule};

use super::codec::{decode_name, encode_name};
use super::text::Text;

/// Render TypeScript imports referenced by one module.
pub(super) fn render_imports(schema: &Schema, module: &SchemaModule, names: &[String]) -> String {
    let mut text = Text::new();

    for item in schema.referenced_types(names) {
        let path = generated_import_path(&module.path, schema.module_path(&item.name));
        text.line(format!("import type {{ {} }} from \"{path}\";", item.name));
    }

    text.finish()
}

/// Render TypeScript imports referenced by one protocol module.
pub(super) fn render_protocol_imports(
    schema: &Schema,
    module: &SchemaModule,
    names: &[String],
) -> String {
    let mut text = Text::new();
    let source = protocol_target_segments(&module.path);
    let serde = runtime_import_path(&source, &["protocol", "serde"]);

    text.line(format!(
        "import {{ Reader, SerdeError, Writer, compareBytes, nestedBytes }} from {:?};",
        serde
    ));

    let referenced = schema.referenced_types(names);
    for item in &referenced {
        let path = protocol_import_path(&module.path, schema.module_path(&item.name));
        text.line(format!("import type {{ {} }} from \"{path}\";", item.name));
    }
    for item in referenced {
        let path = protocol_import_path(&module.path, schema.module_path(&item.name));
        text.line(format!(
            "import {{ {}, {} }} from \"{path}\";",
            decode_name(&item.name),
            encode_name(&item.name)
        ));
    }

    text.finish()
}

/// Return the TypeScript import path between two generated semantic modules.
fn generated_import_path(source: &ModulePath, target: &ModulePath) -> String {
    let source = generated_target_segments(source);
    let target = generated_target_segments(target);

    module_import_path(&source, &target)
}

/// Return the TypeScript import path between two generated protocol modules.
fn protocol_import_path(source: &ModulePath, target: &ModulePath) -> String {
    let source = protocol_target_segments(source);
    let target = protocol_target_segments(target);

    module_import_path(&source, &target)
}

/// Return the TypeScript import path between two generated module paths.
fn module_import_path(source: &[String], target: &[String]) -> String {
    let source_directory = &source[..source.len() - 1];
    let mut shared = 0;

    while shared < source_directory.len()
        && shared < target.len()
        && source_directory[shared] == target[shared]
    {
        shared += 1;
    }

    let mut segments = Vec::new();
    for _ in shared..source_directory.len() {
        segments.push("..".to_string());
    }
    segments.extend(target[shared..].iter().cloned());

    let path = segments.join("/");
    if path.starts_with('.') {
        format!("{path}.js")
    } else {
        format!("./{path}.js")
    }
}

/// Return one TypeScript import path from a generated module to a runtime module.
fn runtime_import_path(source: &[String], target: &[&str]) -> String {
    let source_directory = &source[..source.len() - 1];
    let mut shared = 0;

    while shared < source_directory.len()
        && shared < target.len()
        && source_directory[shared] == target[shared]
    {
        shared += 1;
    }

    let mut segments = Vec::new();
    for _ in shared..source_directory.len() {
        segments.push("..".to_string());
    }
    segments.extend(target[shared..].iter().map(|segment| segment.to_string()));

    let path = segments.join("/");
    if path.starts_with('.') {
        format!("{path}.js")
    } else {
        format!("./{path}.js")
    }
}

/// Return TypeScript generated semantic target segments.
pub(super) fn generated_target_segments(path: &ModulePath) -> Vec<String> {
    let mut segments = Vec::with_capacity(path.segments().len() + 1);
    segments.push("generated".to_string());
    segments.extend(path.segments().iter().cloned());

    segments
}

/// Return TypeScript generated protocol target segments.
fn protocol_target_segments(path: &ModulePath) -> Vec<String> {
    let mut segments = Vec::with_capacity(path.segments().len() + 1);
    segments.push("generated".to_string());
    segments.extend(path.segments().iter().cloned());

    segments
}
