use std::path::Path;

use anyhow::Result;
use destack_rpc::{Limits, ProtocolVersion};

use crate::generate::core::write_text;

use super::text::{GENERATED_HEADER, Text};

pub(super) fn generate_rpc_defaults(root: &Path) -> Result<()> {
    let limits = Limits::default();
    let mut text = Text::new();
    text.line(GENERATED_HEADER);
    text.blank();
    text.line("import type { Limits, Peer } from \"./protocol/handshake.js\";");
    text.line("import type { ProtocolVersion } from \"./protocol/version.js\";");
    text.blank();

    text.doc("Current exact RPC wire grammar.", "");
    text.line(format!(
        "export const protocolVersion: ProtocolVersion = {};",
        ProtocolVersion::CURRENT.0
    ));
    text.blank();

    text.doc("Default RPC resource limits.", "");
    text.line("export const limits: Limits = {");
    text.line(format!(
        "    maxMessageBytes: {}n,",
        limits.max_message_bytes
    ));
    text.line(format!(
        "    maxPayloadBytes: {}n,",
        limits.max_payload_bytes
    ));
    text.line(format!(
        "    maxConcurrentCalls: {},",
        limits.max_concurrent_calls
    ));
    text.line(format!("    streamWindow: {},", limits.stream_window));
    text.line("};");
    text.blank();

    text.doc("Default TypeScript RPC peer description.", "");
    text.line("export const peer: Peer = {");
    text.line("    name: \"destack-typescript\",");
    text.line(format!("    version: {:?},", env!("CARGO_PKG_VERSION")));
    text.line("    buildId: undefined,");
    text.line("};");

    write_text(
        root,
        "client/typescript/src/_generated/rpc/defaults.ts",
        text.finish(),
    )
}
