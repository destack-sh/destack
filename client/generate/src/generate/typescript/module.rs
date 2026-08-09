use crate::generate::schema::{Module, Schema};

use super::codec::render_codec_item;
use super::import::render_imports;
use super::item::render_item;
use super::scope::Scope;
use super::text::{GENERATED_HEADER, Text};

pub(super) fn render_module(schema: &Schema, module: &Module, keys: &[String]) -> String {
    let mut text = Text::new();
    let scope = Scope::new(schema, keys);
    text.line(GENERATED_HEADER);
    text.blank();

    let imports = render_imports(schema, module, keys, &scope);
    if !imports.is_empty() {
        text.raw(imports);
        text.blank();
    }

    for key in keys {
        let item = schema.item(key);
        render_item(schema, item, &scope, &mut text);
        render_codec_item(schema, item, &scope, &mut text);
    }

    text.finish()
}
