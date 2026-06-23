use std::path::Path;

use anyhow::Result;
use destack_workspace::protocol::{
    DEFAULT_MAX_FRAME_BYTES, DEFAULT_MAX_PAYLOAD_BYTES, MIN_PROTOCOL_VERSION, PROTOCOL_VERSION,
};

use crate::generate::core::write_text;

use super::package::render_all;
use super::text::Text;

pub(super) fn generate_protocol_defaults(root: &Path) -> Result<()> {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from .handshake import ClientDescriptor, ProtocolLimits");
    text.line("from .version import ProtocolRange, ProtocolVersion");
    text.blank();
    text.line("protocol_version = ProtocolVersion(");
    text.line(format!("    major={},", PROTOCOL_VERSION.major));
    text.line(format!("    minor={},", PROTOCOL_VERSION.minor));
    text.line(format!("    patch={},", PROTOCOL_VERSION.patch));
    text.line(")");
    text.blank();
    text.line("min_protocol_version = ProtocolVersion(");
    text.line(format!("    major={},", MIN_PROTOCOL_VERSION.major));
    text.line(format!("    minor={},", MIN_PROTOCOL_VERSION.minor));
    text.line(format!("    patch={},", MIN_PROTOCOL_VERSION.patch));
    text.line(")");
    text.blank();
    text.line("protocol_range = ProtocolRange(");
    text.line("    min=min_protocol_version,");
    text.line("    max=protocol_version,");
    text.line(")");
    text.blank();
    text.line("protocol_limits = ProtocolLimits(");
    text.line(format!("    max_frame_bytes={},", DEFAULT_MAX_FRAME_BYTES));
    text.line(format!(
        "    max_payload_bytes={},",
        DEFAULT_MAX_PAYLOAD_BYTES
    ));
    text.line(")");
    text.blank();
    text.line("client_descriptor = ClientDescriptor(");
    text.line("    name=\"destack-python\",");
    text.line(format!("    version={:?},", env!("CARGO_PKG_VERSION")));
    text.line("    build=None,");
    text.line(")");
    text.blank();
    render_all(
        &mut text,
        &[
            "protocol_version".to_string(),
            "min_protocol_version".to_string(),
            "protocol_range".to_string(),
            "protocol_limits".to_string(),
            "client_descriptor".to_string(),
        ],
    );

    write_text(
        root,
        "bridge/python/src/destack/_generated/protocol/defaults.py",
        text.finish(),
    )
}
