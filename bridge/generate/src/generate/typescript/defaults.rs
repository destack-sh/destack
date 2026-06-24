use std::path::Path;

use anyhow::Result;
use destack_workspace::protocol::{
    DEFAULT_MAX_FRAME_BYTES, DEFAULT_MAX_PAYLOAD_BYTES, MIN_PROTOCOL_VERSION, PROTOCOL_VERSION,
};

use crate::generate::core::write_text;

use super::text::{GENERATED_HEADER, Text};

pub(super) fn generate_protocol_defaults(root: &Path) -> Result<()> {
    let mut text = Text::new();
    text.line(GENERATED_HEADER);
    text.blank();
    text.line("import type { ClientDescriptor, ProtocolLimits } from \"./handshake.js\";");
    text.line("import type { ProtocolRange, ProtocolVersion } from \"./version.js\";");
    text.blank();

    text.doc("Current workspace protocol version.", "");
    text.line("export const protocolVersion: ProtocolVersion = {");
    text.line(format!("    major: {},", PROTOCOL_VERSION.major));
    text.line(format!("    minor: {},", PROTOCOL_VERSION.minor));
    text.line(format!("    patch: {},", PROTOCOL_VERSION.patch));
    text.line("};");
    text.blank();

    text.doc("Minimum compatible workspace protocol version.", "");
    text.line("export const minProtocolVersion: ProtocolVersion = {");
    text.line(format!("    major: {},", MIN_PROTOCOL_VERSION.major));
    text.line(format!("    minor: {},", MIN_PROTOCOL_VERSION.minor));
    text.line(format!("    patch: {},", MIN_PROTOCOL_VERSION.patch));
    text.line("};");
    text.blank();

    text.doc("Supported protocol range for TypeScript clients.", "");
    text.line("export const protocolRange: ProtocolRange = {");
    text.line("    min: minProtocolVersion,");
    text.line("    max: protocolVersion,");
    text.line("};");
    text.blank();

    text.doc("Default protocol limits for TypeScript clients.", "");
    text.line("export const protocolLimits: ProtocolLimits = {");
    text.line(format!("    maxFrameBytes: {DEFAULT_MAX_FRAME_BYTES}n,"));
    text.line(format!(
        "    maxPayloadBytes: {DEFAULT_MAX_PAYLOAD_BYTES}n,"
    ));
    text.line("};");
    text.blank();

    text.doc(
        "Default protocol client descriptor for TypeScript clients.",
        "",
    );
    text.line("export const clientDescriptor: ClientDescriptor = {");
    text.line("    name: \"destack-typescript\",");
    text.line(format!("    version: {:?},", env!("CARGO_PKG_VERSION")));
    text.line("};");

    write_text(
        root,
        "bridge/typescript/src/_generated/protocol/defaults.ts",
        text.finish(),
    )
}
