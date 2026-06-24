use std::collections::{BTreeMap, BTreeSet};

use crate::generate::schema::Schema;

use super::name::namespace;
use super::text::{GENERATED_HEADER, Text};

const PUBLIC_ROOTS: &[&str] = &[
    "artifact",
    "core",
    "dir",
    "heap",
    "js",
    "mir",
    "program",
    "qir",
    "query",
    "repository",
    "source",
];

/// One generated TypeScript namespace index file.
pub(super) struct IndexFile {
    /// File path under the workspace root.
    pub(super) path: String,
    /// File content.
    pub(super) content: String,
}

/// Render generated namespace index files.
pub(super) fn render_generated_indexes(schema: &Schema) -> Vec<IndexFile> {
    let tree = IndexTree::new(schema);

    tree.render()
}

/// Render generated public namespace facade files.
pub(super) fn render_public_indexes(schema: &Schema) -> Vec<IndexFile> {
    PublicIndexTree::new(schema).render()
}

/// Render the TypeScript public entrypoint.
pub(super) fn render_index(schema: &Schema) -> String {
    let mut text = Text::new();
    text.line(GENERATED_HEADER);
    text.blank();

    for name in IndexTree::new(schema).roots() {
        let namespace = namespace(&name);
        if is_public_root(&name) {
            text.line(format!(
                "export * as {namespace} from \"./{name}/index.generated.js\";"
            ));
        }
        // fall back to generated namespaces for roots without public facades
        else {
            text.line(format!(
                "export * as {namespace} from \"./_generated/{name}/namespace.js\";"
            ));
        }
    }

    text.blank();
    render_index_runtime_exports(&mut text);

    text.finish()
}

/// TypeScript public facade index tree.
struct PublicIndexTree {
    /// Exported names keyed by public root and source module.
    modules: BTreeMap<String, BTreeMap<Vec<String>, BTreeSet<String>>>,
}

impl PublicIndexTree {
    /// Build one public facade tree.
    fn new(schema: &Schema) -> Self {
        let mut modules = BTreeMap::<String, BTreeMap<Vec<String>, BTreeSet<String>>>::new();

        for module in &schema.modules {
            let Some(root) = module.path.segments().first().cloned() else {
                continue;
            };
            if !is_public_root(&root) {
                continue;
            }
            let module_names = schema.module_names(&module.keys);
            let path = module.path.segments()[1..].to_vec();

            modules
                .entry(root)
                .or_default()
                .entry(path)
                .or_default()
                .extend(module_names);
        }

        Self { modules }
    }

    /// Render all public facade files.
    fn render(&self) -> Vec<IndexFile> {
        let mut outputs = Vec::new();

        for (root, modules) in &self.modules {
            if has_colliding_names(modules) {
                outputs.extend(self.render_nested_root(root, modules));
            } else {
                outputs.push(self.render_flat_root(root, modules));
            }
        }

        outputs
    }

    /// Render one flat public facade file.
    fn render_flat_root(
        &self,
        root: &str,
        modules: &BTreeMap<Vec<String>, BTreeSet<String>>,
    ) -> IndexFile {
        let mut text = Text::new();
        text.line(GENERATED_HEADER);
        text.blank();

        for (path, names) in modules {
            text.line("export {");
            for name in names {
                text.line(format!("    {name},"));
            }
            text.line(format!(
                "}} from {:?};",
                public_generated_import(root, &[], path)
            ));
        }

        render_handwritten_exports(root, &[], &mut text);

        IndexFile {
            path: public_index_path(root, &[]),
            content: text.finish(),
        }
    }

    /// Render nested public facade files for one colliding root.
    fn render_nested_root(
        &self,
        root: &str,
        modules: &BTreeMap<Vec<String>, BTreeSet<String>>,
    ) -> Vec<IndexFile> {
        let mut directories = BTreeSet::<Vec<String>>::new();
        directories.insert(Vec::new());

        for path in modules.keys() {
            for depth in 1..path.len() {
                directories.insert(path[..depth].to_vec());
            }
        }

        directories
            .into_iter()
            .map(|directory| self.render_nested_directory(root, &directory, modules))
            .collect()
    }

    /// Render one nested public facade file.
    fn render_nested_directory(
        &self,
        root: &str,
        directory: &[String],
        modules: &BTreeMap<Vec<String>, BTreeSet<String>>,
    ) -> IndexFile {
        let mut child_directories = BTreeSet::<String>::new();
        let mut direct_modules = Vec::<(&Vec<String>, &BTreeSet<String>)>::new();

        for (path, names) in modules {
            if !path.starts_with(directory) {
                continue;
            }

            if path.len() == directory.len() + 1 {
                direct_modules.push((path, names));
            } else if path.len() > directory.len() + 1 {
                child_directories.insert(path[directory.len()].clone());
            }
        }

        let mut text = Text::new();
        text.line(GENERATED_HEADER);
        text.blank();

        for child in child_directories {
            let namespace = namespace(&child);
            text.line(format!(
                "export * as {namespace} from \"./{child}/index.generated.js\";"
            ));
        }

        for (path, names) in direct_modules {
            text.line("export {");
            for name in names {
                text.line(format!("    {name},"));
            }
            text.line(format!(
                "}} from {:?};",
                public_generated_import(root, directory, path)
            ));
        }

        render_handwritten_exports(root, directory, &mut text);

        IndexFile {
            path: public_index_path(root, directory),
            content: text.finish(),
        }
    }
}

/// TypeScript namespace index tree.
struct IndexTree {
    /// Child directories keyed by directory path.
    directories: BTreeMap<Vec<String>, BTreeSet<String>>,
    /// Child files keyed by directory path.
    files: BTreeMap<Vec<String>, BTreeSet<String>>,
}

impl IndexTree {
    /// Build one namespace index tree.
    fn new(schema: &Schema) -> Self {
        let mut directories = BTreeMap::<Vec<String>, BTreeSet<String>>::new();
        let mut files = BTreeMap::<Vec<String>, BTreeSet<String>>::new();

        for module in &schema.modules {
            let segments = module.path.segments();
            for depth in 0..segments.len() {
                let directory = segments[..depth].to_vec();
                let child = segments[depth].clone();

                if depth + 1 == segments.len() {
                    files.entry(directory).or_default().insert(child);
                } else {
                    directories.entry(directory).or_default().insert(child);
                }
            }
        }

        Self { directories, files }
    }

    /// Return top-level generated namespaces.
    fn roots(&self) -> Vec<String> {
        self.directories
            .get(&Vec::new())
            .into_iter()
            .flat_map(|children| children.iter().cloned())
            .collect()
    }

    /// Render all generated namespace index files.
    fn render(&self) -> Vec<IndexFile> {
        let mut outputs = Vec::new();
        let mut directories = BTreeSet::<Vec<String>>::new();

        directories.extend(
            self.directories
                .keys()
                .filter(|directory| !directory.is_empty())
                .cloned(),
        );
        directories.extend(
            self.files
                .keys()
                .filter(|directory| !directory.is_empty())
                .cloned(),
        );

        for directory in directories {
            outputs.push(IndexFile {
                path: format!(
                    "client/typescript/src/_generated/{}/namespace.ts",
                    directory.join("/")
                ),
                content: self.render_directory(&directory),
            });
        }

        outputs
    }

    /// Render one generated namespace index file.
    fn render_directory(&self, directory: &[String]) -> String {
        let mut text = Text::new();
        text.line(GENERATED_HEADER);
        text.blank();

        if let Some(files) = self.files.get(directory) {
            for file in files {
                let namespace = namespace(file);
                text.line(format!("export * as {namespace} from \"./{file}.js\";"));
            }
        }

        if let Some(directories) = self.directories.get(directory) {
            for directory in directories {
                let namespace = namespace(directory);
                text.line(format!(
                    "export * as {namespace} from \"./{directory}/namespace.js\";"
                ));
            }
        }

        text.finish()
    }
}

/// Return whether one generated root is part of the public facade.
fn is_public_root(root: &str) -> bool {
    PUBLIC_ROOTS.contains(&root)
}

/// Return whether any public names collide in one root facade.
fn has_colliding_names(modules: &BTreeMap<Vec<String>, BTreeSet<String>>) -> bool {
    let mut names = BTreeSet::<String>::new();

    for module_names in modules.values() {
        for name in module_names {
            if !names.insert(name.clone()) {
                return true;
            }
        }
    }

    false
}

/// Return one import path from a public facade to a generated source module.
fn public_generated_import(root: &str, directory: &[String], path: &[String]) -> String {
    let mut segments = Vec::new();

    for _ in 0..=directory.len() {
        segments.push("..".to_string());
    }
    segments.push("_generated".to_string());
    segments.push(root.to_string());
    segments.extend(path.iter().cloned());

    format!("{}.js", segments.join("/"))
}

/// Render handwritten public exports for one root facade.
fn render_handwritten_exports(root: &str, directory: &[String], text: &mut Text) {
    if root == "core" && directory.is_empty() {
        text.blank();
        text.line("export { StringPool } from \"./string.js\";");
    }
    if root == "dir" && directory.is_empty() {
        text.blank();
        text.line("export { BindingTable } from \"./binding.js\";");
    }
}

/// Return one public TypeScript index path.
fn public_index_path(root: &str, directory: &[String]) -> String {
    if directory.is_empty() {
        format!("client/typescript/src/{root}/index.generated.ts")
    } else {
        format!(
            "client/typescript/src/{root}/{}/index.generated.ts",
            directory.join("/")
        )
    }
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
            "type MemoryContent",
            "type MemoryFile",
            "type MemoryWorkspace",
            "type MemoryWorkspaceOptions",
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
        "./_generated/protocol/defaults.js",
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
            "BinaryReader",
            "BinaryWriter",
            "SerdeError",
            "bytesFromJson",
            "bytesToJson",
            "compareBytes",
            "decodeValue",
            "encodeValue",
            "jsonArray",
            "jsonBigint",
            "jsonBool",
            "jsonField",
            "jsonInteger",
            "jsonNumber",
            "jsonObject",
            "jsonOptional",
            "jsonString",
            "nestedBytes",
            "type Json",
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
        "./_generated/protocol/workspace/client.js",
        &["WorkspaceClient"],
    );
    text.export(
        "./_generated/protocol/workspace/command/common.js",
        &[
            "CommandInput",
            "CommandRevision",
            "type CommandEnvVar",
            "type CommandTargetOverrides",
            "type ManifestOverride",
        ],
    );
    text.export(
        "./_generated/protocol/workspace/command/build.js",
        &["type BuildInput", "type BuildOutputs"],
    );
    text.export(
        "./_generated/protocol/workspace/command/check.js",
        &["type CheckInput", "type LintInput"],
    );
    text.export(
        "./_generated/protocol/workspace/command/format.js",
        &["FormatSource", "type FormatInput", "type FormatMode"],
    );
    text.export(
        "./_generated/protocol/workspace/command/output.js",
        &[
            "type BenchOutput",
            "type BuildOutput",
            "type CacheOutput",
            "type CheckOutput",
            "type CleanOutput",
            "type DocOutput",
            "type DoctorOutput",
            "type FormatOutput",
            "type InfoOutput",
            "type LintOutput",
            "type RunOutput",
            "type SettingsOutput",
            "type TargetsOutput",
            "type TaskOutput",
            "type TestOutput",
        ],
    );
    text.export(
        "./_generated/protocol/workspace/command/run.js",
        &["RunMode", "type RunInput"],
    );
    text.export(
        "./_generated/protocol/workspace/command/task.js",
        &["TaskAction", "type TaskInput"],
    );
    text.export(
        "./_generated/protocol/watch.js",
        &[
            "type WatchBatchResponse",
            "type WatchStartedResponse",
            "type WatchStoppedResponse",
            "type WatchStartOptions",
        ],
    );
    text.export(
        "./_generated/protocol/workspace/artifact/export.js",
        &["type ExportRequest", "type ExportResult"],
    );
    text.export("./_generated/protocol/response.js", &["type ArtifactBlob"]);
}
