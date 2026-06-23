use crate::generate::schema::Schema;

use super::text::{GENERATED_HEADER, Text};

pub(super) fn render_index(schema: &Schema) -> String {
    let mut text = Text::new();
    text.line(GENERATED_HEADER);
    text.blank();

    for module in &schema.modules {
        text.line(format!(
            "export * from \"./generated/{}.js\";",
            module.path.slash_path()
        ));
    }

    text.blank();
    render_index_runtime_exports(&mut text);

    text.finish()
}

/// Render runtime exports for the TypeScript public entrypoint.
pub(super) fn render_index_runtime_exports(text: &mut Text) {
    text.export(
        "./workspace/workspace.js",
        &[
            "EmbeddedTransport",
            "RemoteWorkspace",
            "openLocalWorkspace",
            "openRemoteWorkspace",
            "openWorkspace",
            "type LocalWorkspaceOptions",
            "type Workspace",
            "type WorkspaceOptions",
        ],
    );
    text.export(
        "./protocol/codec.js",
        &[
            "decodeFrame",
            "decodeHeader",
            "decodeMessage",
            "encodeFrame",
            "encodeHeader",
            "encodeMessage",
            "ProtocolCodecError",
            "type FrameHeader",
        ],
    );
    text.export(
        "./protocol/connection/index.js",
        &[
            "Connection",
            "connectEndpoint",
            "type NotificationHandler",
            "type ClientOptions",
            "type ConnectionOptions",
        ],
    );
    text.export(
        "./generated/protocol/defaults.js",
        &[
            "clientDescriptor",
            "minProtocolVersion",
            "protocolLimits",
            "protocolRange",
            "protocolVersion",
        ],
    );
    text.export(
        "./protocol/serde.js",
        &[
            "SerdeError",
            "Reader",
            "Writer",
            "compareBytes",
            "decodeValue",
            "encodeValue",
            "nestedBytes",
        ],
    );
    text.export(
        "./protocol/connection/index.js",
        &["TransportError", "WebSocketTransport", "type Transport"],
    );
    text.export(
        "./protocol/workspace.js",
        &[
            "type BenchInputInit",
            "type BuildInputInit",
            "type CacheInputInit",
            "type CheckInputInit",
            "type CleanInputInit",
            "type CommandInputInit",
            "type DocInputInit",
            "type DoctorInputInit",
            "type FormatInputInit",
            "type FormatSourceInit",
            "type InfoInputInit",
            "type LintInputInit",
            "type RemoteWorkspaceOptions",
            "type RunInputInit",
            "type SettingsInputInit",
            "type TargetsInputInit",
            "type TaskInputInit",
            "type TestInputInit",
            "type WatchOptionsInit",
        ],
    );
    text.export(
        "./generated/protocol/workspace/client.js",
        &["WorkspaceClient"],
    );
    text.export(
        "./generated/protocol/workspace/command/common.js",
        &[
            "CommandInput",
            "CommandRevision",
            "type CommandEnvVar",
            "type CommandTargetOverrides",
            "type JsonValue",
            "type ManifestOverride",
        ],
    );
    text.export(
        "./generated/protocol/workspace/command/build.js",
        &["type BuildInput", "type BuildOutputs"],
    );
    text.export(
        "./generated/protocol/workspace/command/check.js",
        &["type CheckInput", "type LintInput"],
    );
    text.export(
        "./generated/protocol/workspace/command/format.js",
        &["FormatSource", "type FormatInput", "type FormatMode"],
    );
    text.export(
        "./generated/protocol/workspace/command/output.js",
        &[
            "type BenchOutput",
            "type BuildOutput as WorkspaceBuildOutput",
            "type CacheOutput",
            "type CheckOutput as WorkspaceCheckOutput",
            "type CleanOutput",
            "type DocOutput",
            "type DoctorOutput",
            "type FormatOutput as WorkspaceFormatOutput",
            "type InfoOutput",
            "type LintOutput as WorkspaceLintOutput",
            "type RunOutput",
            "type SettingsOutput",
            "type TargetsOutput",
            "type TaskOutput",
            "type TestOutput",
        ],
    );
    text.export(
        "./generated/protocol/workspace/command/run.js",
        &["RunMode", "type RunInput"],
    );
    text.export(
        "./generated/protocol/workspace/command/task.js",
        &["TaskAction", "type TaskInput"],
    );
    text.export(
        "./generated/protocol/watch.js",
        &[
            "type WatchBatchResponse",
            "type WatchStartedResponse",
            "type WatchStoppedResponse",
            "type WatchStartOptions",
        ],
    );
    text.export(
        "./generated/protocol/workspace/artifact/export.js",
        &["type ExportRequest", "type ExportResult"],
    );
    text.export("./generated/protocol/response.js", &["type ArtifactBlob"]);
}
