use crate::generate::schema::{Schema, SchemaModule};

use super::codec::{render_protocol_codec_item, render_protocol_codec_stub_item};
use super::item::render_protocol_item;
use super::package::render_all;
use super::path::{protocol_module_names, render_protocol_imports};
use super::text::Text;

pub(super) fn render_protocol_module(schema: &Schema, module: &SchemaModule) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from collections.abc import Mapping, Sequence");
    text.line("from dataclasses import dataclass");
    text.line("from typing import TYPE_CHECKING, Any, Literal, TypeAlias");
    text.blank();
    text.line("from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes");
    text.blank();
    text.raw(render_protocol_imports(schema, module));

    for name in &module.names {
        let item = schema.item(name);
        text.raw(render_protocol_item(schema, item));
        text.raw(render_protocol_codec_item(schema, module, item));
    }

    render_all(&mut text, &protocol_module_names(schema, module));

    text.finish()
}

/// Render one generated Python protocol module stub.
pub(super) fn render_protocol_module_stub(schema: &Schema, module: &SchemaModule) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from collections.abc import Mapping, Sequence");
    text.line("from dataclasses import dataclass");
    text.line("from typing import TYPE_CHECKING, Any, Literal, TypeAlias");
    text.blank();
    text.line("from destack.protocol.serde import Reader, Writer");
    text.blank();
    text.raw(render_protocol_imports(schema, module));

    for name in &module.names {
        let item = schema.item(name);
        text.raw(render_protocol_item(schema, item));
        text.raw(render_protocol_codec_stub_item(item));
    }

    render_all(&mut text, &protocol_module_names(schema, module));

    text.finish()
}
