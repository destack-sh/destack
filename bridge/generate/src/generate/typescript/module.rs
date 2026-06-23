use crate::generate::schema::{Schema, SchemaModule};

use super::codec::render_codec_item;
use super::item::render_item;
use super::path::{render_imports, render_protocol_imports};
use super::text::{GENERATED_HEADER, Text};

pub(super) fn render_module(schema: &Schema, module: &SchemaModule, names: &[String]) -> String {
    let mut text = Text::new();
    text.line(GENERATED_HEADER);
    text.blank();

    let imports = render_imports(schema, module, names);

    if !imports.is_empty() {
        text.raw(imports);
        text.blank();
    }

    for name in names {
        let item = schema.item(name);
        render_item(schema, item, &mut text);
    }

    text.finish()
}

/// Render one generated TypeScript protocol module.
pub(super) fn render_protocol_module(
    schema: &Schema,
    module: &SchemaModule,
    names: &[String],
) -> String {
    let mut text = Text::new();
    text.line(GENERATED_HEADER);
    text.blank();

    let imports = render_protocol_imports(schema, module, names);
    if !imports.is_empty() {
        text.raw(imports);
        text.blank();
    }

    for name in names {
        let item = schema.item(name);
        render_item(schema, item, &mut text);
        render_codec_item(schema, item, &mut text);
    }

    text.finish()
}
