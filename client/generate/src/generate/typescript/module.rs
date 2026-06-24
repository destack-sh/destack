use crate::generate::implementation::implementation_owners;
use crate::generate::schema::{Schema, SchemaModule};

use super::codec::render_codec_item;
use super::item::render_item;
use super::path::{TypeNames, render_imports};
use super::text::{GENERATED_HEADER, Text};

pub(super) fn render_module(schema: &Schema, module: &SchemaModule, names: &[String]) -> String {
    let mut text = Text::new();
    let type_names = TypeNames::new(schema, names);
    text.line(GENERATED_HEADER);
    text.blank();

    let imports = render_imports(schema, module, names, &type_names);
    let implementation_imports = render_implementation_imports(module);

    if !imports.is_empty() {
        text.raw(imports);
        text.blank();
    }

    if !implementation_imports.is_empty() {
        text.raw(implementation_imports);
        text.blank();
    }

    for name in names {
        let item = schema.item(name);
        render_item(schema, module, item, &type_names, &mut text);
        render_codec_item(schema, item, &type_names, &mut text);
    }

    text.finish()
}

/// Render TypeScript implementation imports for one generated module.
fn render_implementation_imports(module: &SchemaModule) -> String {
    let mut text = Text::new();

    for owner in implementation_owners(module) {
        let import = typescript_implementation_import(module, owner.module());
        text.line(format!(
            "import {{ {} }} from {:?};",
            owner.typescript_impl(),
            import
        ));
    }

    text.finish()
}

/// Return one implementation import path relative to one generated TypeScript module.
fn typescript_implementation_import(
    module: &SchemaModule,
    implementation_module: &[&str],
) -> String {
    let mut segments = vec![".."; module.path.segments().len()];
    segments.push("_impl");

    let mut segments = segments.into_iter().map(str::to_string).collect::<Vec<_>>();
    segments.extend(
        implementation_module
            .iter()
            .map(|segment| segment.to_string()),
    );

    format!("{}.js", segments.join("/"))
}
