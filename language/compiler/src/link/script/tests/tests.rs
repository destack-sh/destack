use std::path::{Path, PathBuf};

use base64::Engine as _;
use destack_artifact::{
    ArtifactKey, BuildManifest, BuildManifestFile, BuildManifestFileType, BuildManifestLoader,
    OutputContent, OutputFile, PackageAssembly, PackageOutput, SourceMapArtifact, TargetOutputName,
};
use destack_source::{DiffOptions, FileType, ModuleId, PackageId, TargetId, Uri, print_diff};
use destack_workspace::{BundleMode, SourceMapMode, Target, TargetDiscovery};
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::compile::CompilePhase;
use crate::link::{OutputId, ScriptLinker};

use super::super::plan::OutputKind;

pub(super) use crate::tests::TestProgram;

const LINKEI_ENTRY_PATH: &str = "dist/js.js";
const LINKEI_MAP_PATH: &str = "dist/js.js.map";
const LINKEI_MANIFEST_PATH: &str = "dist/js.manifest.json";

/// One expected manifest chunk file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExpectedManifestChunk {
    /// The manifest record being built.
    file: BuildManifestFile,
}

/// Build one linked entry file.
pub(super) fn linked_entry(text: &str) -> LinkedTextFile {
    LinkedTextFile {
        path: LINKEI_ENTRY_PATH.to_string(),
        file_type: FileType::JavaScript,
        text: text.to_string(),
    }
}

/// Build one linked entry file with one appended source map reference.
pub(super) fn linked_entry_with_source_map(
    text: &str,
    source_map_reference: &str,
) -> LinkedTextFile {
    let mut text = text.to_string();

    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }

    text.push_str(&format!("//# sourceMappingURL={source_map_reference}\n"));

    linked_entry(&text)
}

/// Build one linked manifest file.
pub(super) fn linked_manifest(value: BuildManifest) -> LinkedJsonFile<BuildManifest> {
    let text = serde_json::to_string_pretty(&value)
        .unwrap_or_else(|error| panic!("failed to serialize expected manifest: {error}"));
    let text = format!("{text}\n");

    LinkedJsonFile {
        path: LINKEI_MANIFEST_PATH.to_string(),
        file_type: FileType::Json,
        text,
        value,
    }
}

/// Build one linked manifest file from one file list.
pub(super) fn manifest(
    files: impl IntoIterator<Item = BuildManifestFile>,
) -> LinkedJsonFile<BuildManifest> {
    linked_manifest(BuildManifest {
        index: None,
        files: files.into_iter().collect(),
    })
}

/// Build one manifest file record.
pub(super) fn manifest_file(
    path: &str,
    file_type: BuildManifestFileType,
    loader: BuildManifestLoader,
) -> BuildManifestFile {
    BuildManifestFile {
        path: path.to_string(),
        r#type: file_type,
        loader,
        name: None,
        input: None,
        is_entry: None,
        is_dynamic_entry: None,
        imports: Vec::new(),
        dynamic_imports: Vec::new(),
        stylesheets: Vec::new(),
    }
}

/// Build one manifest chunk record.
pub(super) fn manifest_chunk(path: &str, name: &str) -> ExpectedManifestChunk {
    ExpectedManifestChunk {
        file: BuildManifestFile {
            path: path.to_string(),
            r#type: BuildManifestFileType::Chunk,
            loader: BuildManifestLoader::Js,
            name: Some(name.to_string()),
            input: None,
            is_entry: None,
            is_dynamic_entry: None,
            imports: Vec::new(),
            dynamic_imports: Vec::new(),
            stylesheets: Vec::new(),
        },
    }
}

/// Build one manifest source map asset record.
pub(super) fn manifest_map(path: &str) -> BuildManifestFile {
    manifest_file(path, BuildManifestFileType::Asset, BuildManifestLoader::Map)
}

/// Build one linked source map file at one exact path.
pub(super) fn map_output(
    path: &str,
    value: SourceMapArtifact,
) -> LinkedJsonFile<SourceMapArtifact> {
    let text = serde_json::to_string(&value)
        .unwrap_or_else(|error| panic!("failed to serialize expected source map: {error}"));
    let text = format!("{text}\n");

    LinkedJsonFile {
        path: path.to_string(),
        file_type: FileType::SourceMap,
        text,
        value,
    }
}

/// Build one linked source map file for the default single-file target path.
pub(super) fn linked_map(value: SourceMapArtifact) -> LinkedJsonFile<SourceMapArtifact> {
    map_output(LINKEI_MAP_PATH, value)
}

/// Build one source map payload.
pub(super) fn source_map(sources: &[&str], mappings: &str) -> SourceMapArtifact {
    SourceMapArtifact {
        version: destack_artifact::SOURCE_MAP_VERSION,
        file: None,
        source_root: None,
        sources: sources.iter().map(|source| (*source).to_string()).collect(),
        sources_content: None,
        names: Vec::new(),
        mappings: mappings.to_string(),
        debug_id: None,
    }
}

/// Build one JavaScript text output.
pub(super) fn script_output(path: &str, text: &str) -> LinkedTextFile {
    LinkedTextFile {
        path: path.to_string(),
        file_type: FileType::JavaScript,
        text: text.to_string(),
    }
}

/// Build one inline source map reference for one map payload.
pub(super) fn inline_source_map_reference(value: &SourceMapArtifact) -> String {
    let source_map = serde_json::to_string(value)
        .unwrap_or_else(|error| panic!("failed to serialize expected inline source map: {error}"));
    let encoded = base64::engine::general_purpose::STANDARD.encode(source_map.as_bytes());

    format!("data:application/json;charset=utf-8;base64,{encoded}")
}

/// One normalized text output file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedTextFile {
    /// The package-relative output path.
    pub path: String,
    /// The emitted file type.
    pub file_type: FileType,
    /// The emitted text.
    pub text: String,
}

/// One normalized JSON output file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedJsonFile<T> {
    /// The package-relative output path.
    pub path: String,
    /// The emitted file type.
    pub file_type: FileType,
    /// The serialized JSON text.
    pub text: String,
    /// The parsed JSON value.
    pub value: T,
}

/// One normalized linked script target assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedScriptTarget {
    /// The target assembly mode.
    pub assembly: PackageAssembly,
    /// The exact output groups and package-relative paths.
    pub output_groups: IndexMap<TargetOutputName, Vec<String>>,
    /// The primary entry output.
    pub entry: LinkedTextFile,
    /// The emitted manifest output.
    pub manifest: LinkedJsonFile<BuildManifest>,
    /// The emitted source map output when one exists.
    pub map: Option<LinkedJsonFile<SourceMapArtifact>>,
}

/// One normalized linked chunked script target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedChunkedScriptTarget {
    /// The target assembly mode.
    pub assembly: PackageAssembly,
    /// The exact output groups and package-relative paths.
    pub output_groups: IndexMap<TargetOutputName, Vec<String>>,
    /// The emitted manifest output.
    pub manifest: LinkedJsonFile<BuildManifest>,
    /// The emitted module outputs keyed by package-relative path.
    pub modules: IndexMap<String, LinkedTextFile>,
}

/// One normalized planned script output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlannedOutput {
    /// The stable output name.
    pub name: String,
    /// The emitted output kind.
    pub kind: OutputKind,
    /// The package-relative member module paths.
    pub modules: Vec<String>,
    /// The outgoing static output dependency names.
    pub static_dependencies: Vec<String>,
    /// The outgoing dynamic output dependency names.
    pub dynamic_dependencies: Vec<String>,
}

/// Build one planned script output.
pub(super) fn planned_output(name: &str, kind: OutputKind, modules: &[&str]) -> PlannedOutput {
    PlannedOutput {
        name: name.to_string(),
        kind,
        modules: modules.iter().map(|module| (*module).to_string()).collect(),
        static_dependencies: Vec::new(),
        dynamic_dependencies: Vec::new(),
    }
}

impl ExpectedManifestChunk {
    /// Set the manifest input path for this chunk.
    pub(super) fn input(mut self, path: &str) -> Self {
        self.file.input = Some(path.to_string());

        self
    }

    /// Mark this chunk as a static entry.
    pub(super) fn entry(mut self) -> Self {
        self.file.is_entry = Some(true);
        self.file.is_dynamic_entry = Some(false);

        self
    }

    /// Mark this chunk as a shared non-entry output.
    pub(super) fn shared(mut self) -> Self {
        self.file.is_entry = Some(false);
        self.file.is_dynamic_entry = Some(false);

        self
    }

    /// Mark this chunk as a dynamic entry.
    pub(super) fn dynamic_entry(mut self) -> Self {
        self.file.is_entry = Some(false);
        self.file.is_dynamic_entry = Some(true);

        self
    }

    /// Set the static imports for this chunk.
    pub(super) fn imports(mut self, imports: &[&str]) -> Self {
        self.file.imports = imports.iter().map(|path| (*path).to_string()).collect();

        self
    }

    /// Set the dynamic imports for this chunk.
    pub(super) fn dynamic_imports(mut self, imports: &[&str]) -> Self {
        self.file.dynamic_imports = imports.iter().map(|path| (*path).to_string()).collect();

        self
    }

    /// Set the associated stylesheet references for this chunk.
    pub(super) fn stylesheets(mut self, stylesheets: &[&str]) -> Self {
        self.file.stylesheets = stylesheets.iter().map(|path| (*path).to_string()).collect();

        self
    }
}

impl From<ExpectedManifestChunk> for BuildManifestFile {
    fn from(chunk: ExpectedManifestChunk) -> Self {
        chunk.file
    }
}

impl PlannedOutput {
    /// Set the static output dependencies for this planned output.
    pub(super) fn static_dependencies(mut self, dependencies: &[&str]) -> Self {
        self.static_dependencies = dependencies
            .iter()
            .map(|dependency| (*dependency).to_string())
            .collect();

        self
    }

    /// Set the dynamic output dependencies for this planned output.
    pub(super) fn dynamic_dependencies(mut self, dependencies: &[&str]) -> Self {
        self.dynamic_dependencies = dependencies
            .iter()
            .map(|dependency| (*dependency).to_string())
            .collect();

        self
    }
}

/// Normalize one raw script block without forcing a trailing newline.
pub(super) fn js(text: &str) -> String {
    normalize_script_block(text, false)
}

/// Normalize one raw script block and append one trailing newline.
pub(super) fn js_output(text: &str) -> String {
    normalize_script_block(text, true)
}

/// Build one linked chunked script target from exact output groups and module outputs.
pub(super) fn chunked_script_target(
    output_groups: IndexMap<TargetOutputName, Vec<String>>,
    manifest: LinkedJsonFile<BuildManifest>,
    modules: impl IntoIterator<Item = LinkedTextFile>,
) -> LinkedChunkedScriptTarget {
    let modules = modules
        .into_iter()
        .map(|file| (file.path.clone(), file))
        .collect::<IndexMap<_, _>>();

    LinkedChunkedScriptTarget {
        assembly: PackageAssembly::Chunked,
        output_groups,
        manifest,
        modules,
    }
}

/// Normalize one raw script block by trimming shared indentation.
fn normalize_script_block(text: &str, is_trailing_newline: bool) -> String {
    // outer spacing
    let text = text.strip_prefix('\n').unwrap_or(text);
    let text = if is_trailing_newline {
        text
    } else {
        text.strip_suffix('\n').unwrap_or(text)
    };

    // shared indentation
    let indentation = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.chars()
                .take_while(|character| matches!(character, ' ' | '\t'))
                .count()
        })
        .min()
        .unwrap_or(0);
    let lines = text
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line.chars().skip(indentation).collect()
            }
        })
        .collect::<Vec<String>>();
    let text = lines.join("\n");

    // trailing newline
    if is_trailing_newline && !text.ends_with('\n') {
        return format!("{text}\n");
    }

    text
}

impl TestProgram {
    /// Build one single-file JavaScript entry with no manifest or source map side products.
    pub(super) fn link_single_file_js_entry_with<F>(
        &self,
        module_id: ModuleId,
        name: &str,
        configure: F,
    ) -> LinkedTextFile
    where
        F: FnOnce(&mut Target),
    {
        let package_id = self.program.module_descriptor(module_id).package_id;
        let target_id = TargetId::new(package_id, name);

        self.configure_single_file_js_target(module_id, name);
        self.configure_target(module_id, name, |target| {
            target.source_map_mode = None;
            target.bundle_output.sourcemap = None;
            target.bundle_output.manifest = false;
            configure(target);
        });
        self.run(ArtifactKey::package_output(package_id, target_id));

        // linker tests should stay free of linker phase diagnostics
        self.check_no_diagnostics_for_phases(&[CompilePhase::Link]);

        let output = self.package_output(package_id, name);

        self.single_text_output(package_id, &output, TargetOutputName::Entry)
    }

    /// Build one single-file JavaScript entry with no manifest or source map side products.
    pub(super) fn link_single_file_js_entry(
        &self,
        module_id: ModuleId,
        name: &str,
    ) -> LinkedTextFile {
        self.link_single_file_js_entry_with(module_id, name, |_| {})
    }

    /// Add multiple modules and return their ids in the same order.
    pub(super) fn add_modules<const N: usize, S>(&self, modules: [(&str, S); N]) -> [ModuleId; N]
    where
        S: AsRef<str>,
    {
        modules.map(|(path, source)| self.add_module(path, source.as_ref()))
    }

    /// Configure one single-file JavaScript target for linker tests.
    pub(super) fn configure_single_file_js_target(&self, module_id: ModuleId, name: &str) {
        let entry_path = {
            let module = self.program.module_descriptor(module_id);
            self.normalize_uri_path(module.package_id, &module.uri)
        };

        self.configure_target(module_id, name, |target| {
            // entry based linking
            target.discovery = TargetDiscovery::Entry;
            target.entry = vec![PathBuf::from(&entry_path)];

            // single-file bundle surface
            target.out_file = Some(PathBuf::from(format!("dist/{name}.js")));

            // manifest and external maps
            target.bundle_output.manifest = true;
            target.bundle_output.sourcemap = Some(SourceMapMode::External);
        });
    }

    /// Configure one chunked JavaScript target for linker tests.
    pub(super) fn configure_chunked_js_target(&self, module_ids: &[ModuleId], name: &str) {
        let entry_paths = module_ids
            .iter()
            .map(|module_id| {
                let module = self.program.module_descriptor(*module_id);
                self.normalize_uri_path(module.package_id, &module.uri)
            })
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        self.configure_target(module_ids[0], name, |target| {
            // entry based linking
            target.discovery = TargetDiscovery::Entry;
            target.entry = entry_paths;

            // chunked bundle surface
            target.out_dir = PathBuf::from("dist");
            target.assembly = BundleMode::Chunked;

            // manifest and external maps
            target.bundle_output.manifest = true;
            target.bundle_output.sourcemap = Some(SourceMapMode::External);
        });
    }

    /// Return the package-relative path for one module.
    pub(super) fn module_relative_path(&self, module_id: ModuleId) -> String {
        let module = self.program.module_descriptor(module_id);

        self.normalize_uri_path(module.package_id, &module.uri)
    }

    /// Build one single-file JavaScript target and return the normalized linked assembly.
    pub(super) fn link_single_file_js_target(
        &self,
        module_id: ModuleId,
        name: &str,
    ) -> LinkedScriptTarget {
        self.link_single_file_js_target_with(module_id, name, |_| {})
    }

    /// Build one single-file JavaScript target with extra configuration and return the linked assembly.
    pub(super) fn link_single_file_js_target_with<F>(
        &self,
        module_id: ModuleId,
        name: &str,
        configure: F,
    ) -> LinkedScriptTarget
    where
        F: FnOnce(&mut Target),
    {
        let package_id = self.program.module_descriptor(module_id).package_id;
        let target_id = TargetId::new(package_id, name);

        self.configure_single_file_js_target(module_id, name);
        self.configure_target(module_id, name, configure);
        self.run(ArtifactKey::package_output(package_id, target_id));

        // linker tests should stay free of linker phase diagnostics
        self.check_no_diagnostics_for_phases(&[CompilePhase::Link]);

        let output = self.package_output(package_id, name);
        self.linked_script_target(package_id, &output)
    }

    /// Build one chunked JavaScript target with extra configuration and return the linked output.
    pub(super) fn link_chunked_js_target_with<F>(
        &self,
        module_ids: &[ModuleId],
        name: &str,
        configure: F,
    ) -> PackageOutput
    where
        F: FnOnce(&mut Target),
    {
        let package_id = self.program.module_descriptor(module_ids[0]).package_id;
        let target_id = TargetId::new(package_id, name);

        self.configure_chunked_js_target(module_ids, name);
        self.configure_target(module_ids[0], name, configure);
        self.run(ArtifactKey::package_output(package_id, target_id));

        // linker tests should stay free of linker phase diagnostics
        self.check_no_diagnostics_for_phases(&[CompilePhase::Link]);

        self.package_output(package_id, name)
    }

    /// Build one chunked JavaScript target and return the normalized linked target.
    pub(super) fn linked_chunked_js_target_with<F>(
        &self,
        module_ids: &[ModuleId],
        name: &str,
        configure: F,
    ) -> LinkedChunkedScriptTarget
    where
        F: FnOnce(&mut Target),
    {
        let package_id = self.program.module_descriptor(module_ids[0]).package_id;
        let output = self.link_chunked_js_target_with(module_ids, name, configure);

        self.linked_chunked_script_target(package_id, &output)
    }

    /// Build one chunked JavaScript target and return the normalized output graph.
    pub(super) fn plan_chunked_js_target_with<F>(
        &self,
        module_ids: &[ModuleId],
        name: &str,
        configure: F,
    ) -> Vec<PlannedOutput>
    where
        F: FnOnce(&mut Target),
    {
        let package_id = self.program.module_descriptor(module_ids[0]).package_id;
        let target_id = TargetId::new(package_id, name);

        self.configure_chunked_js_target(module_ids, name);
        self.configure_target(module_ids[0], name, configure);
        self.run(ArtifactKey::package_output(package_id, target_id.clone()));

        // linker tests should stay free of linker phase diagnostics
        self.check_no_diagnostics_for_phases(&[CompilePhase::Link]);

        let package = self.program.package_descriptor(package_id);
        let package_dir = package
            .path
            .clone()
            .unwrap_or_else(|| self.program.root_directory().clone());
        let target = package
            .targets
            .get(&target_id)
            .cloned()
            .unwrap_or_else(|| panic!("missing target '{name}'"));
        let context = crate::tests::test_provider_context(
            self.compiler.as_ref(),
            self.current_revision(),
            ArtifactKey::WorkspaceLinted,
        );
        let linker = ScriptLinker::new(
            self.compiler.as_ref(),
            context.as_ref(),
            &package_dir,
            None,
            &target,
            &target_id,
            package_id,
        );
        let linked_modules = linker
            .require_module_artifacts(module_ids)
            .unwrap_or_else(|error| panic!("failed to require script target artifacts: {error:?}"));

        let module_set = linker
            .build_script_module_set(module_ids, &linked_modules)
            .unwrap_or_else(|error| panic!("failed to build script module set: {error:?}"));
        let output_graph = linker
            .build_script_output_graph(&module_set)
            .unwrap_or_else(|error| panic!("failed to build script output graph: {error:?}"));
        let output_layout = linker
            .build_output_layout(&output_graph)
            .unwrap_or_else(|error| panic!("failed to build script output layout: {error:?}"));

        output_graph
            .outputs()
            .iter()
            .enumerate()
            .map(|(output_index, output)| PlannedOutput {
                name: output_layout
                    .output_name(OutputId(output_index))
                    .unwrap_or_else(|| panic!("missing output name for output id {output_index}"))
                    .to_string(),
                kind: output.space(),
                modules: output
                    .modules()
                    .iter()
                    .map(|module_id| self.module_relative_path(*module_id))
                    .collect(),
                static_dependencies: output
                    .static_output_dependencies()
                    .iter()
                    .filter_map(|output_id| output_layout.output_name(*output_id))
                    .map(ToString::to_string)
                    .collect(),
                dynamic_dependencies: output
                    .dynamic_output_dependencies()
                    .iter()
                    .filter_map(|output_id| output_layout.output_name(*output_id))
                    .map(ToString::to_string)
                    .collect(),
            })
            .collect()
    }

    /// Assert one linked script target exactly.
    pub(super) fn assert_linked_script_target(
        &self,
        actual: &LinkedScriptTarget,
        expected: &LinkedScriptTarget,
    ) {
        self.assert_linked_script_assembly(actual, expected.assembly);
        self.assert_linked_script_output_groups(actual, &expected.output_groups);
        self.assert_linked_script_entry(actual, &expected.entry);
        self.assert_linked_script_manifest(actual, &expected.manifest);
        self.assert_linked_script_map(actual, expected.map.as_ref());
    }

    /// Assert the assembly mode for one linked script target.
    pub(super) fn assert_linked_script_assembly(
        &self,
        actual: &LinkedScriptTarget,
        expected: PackageAssembly,
    ) {
        assert_eq!(actual.assembly, expected, "linked script assembly mismatch");
    }

    /// Assert the output groups for one linked script target.
    pub(super) fn assert_linked_script_output_groups(
        &self,
        actual: &LinkedScriptTarget,
        expected: &IndexMap<TargetOutputName, Vec<String>>,
    ) {
        self.assert_output_groups(&actual.output_groups, expected, "linked output groups");
    }

    /// Assert the primary entry output for one linked script target.
    pub(super) fn assert_linked_script_entry(
        &self,
        actual: &LinkedScriptTarget,
        expected: &LinkedTextFile,
    ) {
        self.assert_linked_text_file(&actual.entry, expected, "linked entry output");
    }

    /// Assert the manifest output for one linked script target.
    pub(super) fn assert_linked_script_manifest(
        &self,
        actual: &LinkedScriptTarget,
        expected: &LinkedJsonFile<BuildManifest>,
    ) {
        self.assert_linked_json_file(&actual.manifest, expected, "linked manifest output");
    }

    /// Assert the source map output for one linked script target.
    pub(super) fn assert_linked_script_map(
        &self,
        actual: &LinkedScriptTarget,
        expected: Option<&LinkedJsonFile<SourceMapArtifact>>,
    ) {
        match (&actual.map, expected) {
            (Some(actual), Some(expected)) => {
                self.assert_linked_json_file(actual, expected, "linked source map output");
            }
            (None, None) => {}
            (Some(actual), None) => {
                panic!("unexpected linked source map output: {}", actual.path);
            }
            (None, Some(expected)) => {
                panic!("missing linked source map output: {}", expected.path);
            }
        }
    }

    /// Assert one linked chunked script target exactly.
    pub(super) fn assert_linked_chunked_script_target(
        &self,
        actual: &LinkedChunkedScriptTarget,
        expected: &LinkedChunkedScriptTarget,
    ) {
        self.assert_linked_chunked_script_assembly(actual, expected.assembly);
        self.assert_output_groups(
            &actual.output_groups,
            &expected.output_groups,
            "linked chunked output groups",
        );
        self.assert_linked_json_file(
            &actual.manifest,
            &expected.manifest,
            "linked chunked manifest output",
        );
        self.assert_linked_chunked_script_modules(&actual.modules, &expected.modules);
    }

    /// Assert the assembly mode for one linked chunked script target.
    pub(super) fn assert_linked_chunked_script_assembly(
        &self,
        actual: &LinkedChunkedScriptTarget,
        expected: PackageAssembly,
    ) {
        assert_eq!(
            actual.assembly, expected,
            "linked chunked assembly mismatch"
        );
    }

    /// Normalize one linked script target into exact assertion data.
    fn linked_script_target(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
    ) -> LinkedScriptTarget {
        let output_groups = output
            .outputs
            .iter()
            .map(|(name, files)| {
                let files = files
                    .iter()
                    .map(|file| self.normalize_uri_path(package_id, &file.uri))
                    .collect::<Vec<_>>();
                (*name, files)
            })
            .collect::<IndexMap<_, _>>();

        LinkedScriptTarget {
            assembly: output.assembly,
            output_groups,
            entry: self.single_text_output(package_id, output, TargetOutputName::Entry),
            manifest: self.single_json_output(package_id, output, TargetOutputName::Manifest),
            map: self.optional_json_output(package_id, output, TargetOutputName::Maps),
        }
    }

    /// Normalize one linked chunked script target into exact assertion data.
    pub(super) fn linked_chunked_script_target(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
    ) -> LinkedChunkedScriptTarget {
        LinkedChunkedScriptTarget {
            assembly: output.assembly,
            output_groups: self.output_group_paths(package_id, output),
            manifest: self.single_json_output(package_id, output, TargetOutputName::Manifest),
            modules: self.text_outputs_from_groups(
                package_id,
                output,
                &[TargetOutputName::Module, TargetOutputName::Entry],
            ),
        }
    }

    /// Return one normalized text output file from the given group.
    fn single_text_output(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
    ) -> LinkedTextFile {
        let file = self.single_output_file(output, output_name);

        match &file.content {
            OutputContent::Text { code, file_type } => LinkedTextFile {
                path: self.normalize_uri_path(package_id, &file.uri),
                file_type: *file_type,
                text: code.clone(),
            },
            _ => panic!("expected text output in '{}'", output_name.as_str()),
        }
    }

    /// Return one normalized text output file addressed by package-relative path.
    pub(super) fn linked_text_output_at_path(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
        expected_path: &str,
    ) -> LinkedTextFile {
        let files = output
            .outputs
            .get(&output_name)
            .unwrap_or_else(|| panic!("missing output group '{}'", output_name.as_str()));
        let file = files
            .iter()
            .find(|file| self.normalize_uri_path(package_id, &file.uri) == expected_path)
            .unwrap_or_else(|| panic!("missing output file '{}'", expected_path));

        match &file.content {
            OutputContent::Text { code, file_type } => LinkedTextFile {
                path: self.normalize_uri_path(package_id, &file.uri),
                file_type: *file_type,
                text: code.clone(),
            },
            _ => panic!("expected text output in '{}'", output_name.as_str()),
        }
    }

    /// Assert one exact text output at one package-relative path.
    pub(super) fn assert_text_output_at_path(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
        expected: &LinkedTextFile,
        noun: &str,
    ) {
        let actual =
            self.linked_text_output_at_path(package_id, output, output_name, &expected.path);

        self.assert_linked_text_file(&actual, expected, noun);
    }

    /// Return one normalized json output file addressed by package-relative path.
    pub(super) fn linked_json_output_at_path<T>(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
        expected_path: &str,
    ) -> LinkedJsonFile<T>
    where
        T: DeserializeOwned,
    {
        let files = output
            .outputs
            .get(&output_name)
            .unwrap_or_else(|| panic!("missing output group '{}'", output_name.as_str()));
        let file = files
            .iter()
            .find(|file| self.normalize_uri_path(package_id, &file.uri) == expected_path)
            .unwrap_or_else(|| panic!("missing output file '{}'", expected_path));

        match &file.content {
            OutputContent::Json {
                content,
                value: _,
                file_type,
            } => LinkedJsonFile {
                path: self.normalize_uri_path(package_id, &file.uri),
                file_type: *file_type,
                text: content.clone(),
                value: serde_json::from_str(content).unwrap_or_else(|error| {
                    panic!(
                        "failed to decode '{}' json output at '{}': {error}",
                        output_name.as_str(),
                        expected_path,
                    )
                }),
            },
            _ => panic!("expected json output in '{}'", output_name.as_str()),
        }
    }

    /// Assert one exact json output at one package-relative path.
    pub(super) fn assert_json_output_at_path<T>(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
        expected: &LinkedJsonFile<T>,
        noun: &str,
    ) where
        T: PartialEq + Serialize + std::fmt::Debug + DeserializeOwned,
    {
        let actual =
            self.linked_json_output_at_path(package_id, output, output_name, &expected.path);

        self.assert_linked_json_file(&actual, expected, noun);
    }

    /// Return one normalized JSON output file from the given group.
    pub(super) fn single_json_output<T>(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
    ) -> LinkedJsonFile<T>
    where
        T: DeserializeOwned,
    {
        let file = self.single_output_file(output, output_name);

        match &file.content {
            OutputContent::Json {
                content,
                value: _,
                file_type,
            } => LinkedJsonFile {
                path: self.normalize_uri_path(package_id, &file.uri),
                file_type: *file_type,
                text: content.clone(),
                value: serde_json::from_str(content).unwrap_or_else(|error| {
                    panic!(
                        "failed to decode '{}' json output: {error}",
                        output_name.as_str()
                    )
                }),
            },
            _ => panic!("expected json output in '{}'", output_name.as_str()),
        }
    }

    /// Return the exact package-relative file paths for each output group.
    pub(super) fn output_group_paths(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
    ) -> IndexMap<TargetOutputName, Vec<String>> {
        output
            .outputs
            .iter()
            .map(|(name, files)| {
                let files = files
                    .iter()
                    .map(|file| self.normalize_uri_path(package_id, &file.uri))
                    .collect::<Vec<_>>();

                (*name, files)
            })
            .collect()
    }

    /// Return one normalized optional JSON output file from the given group.
    fn optional_json_output<T>(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_name: TargetOutputName,
    ) -> Option<LinkedJsonFile<T>>
    where
        T: DeserializeOwned,
    {
        let files = output.outputs.get(&output_name)?;
        let [file] = files.as_slice() else {
            panic!(
                "expected exactly one file in optional output group '{}'",
                output_name.as_str()
            );
        };

        match &file.content {
            OutputContent::Json {
                content,
                value: _,
                file_type,
            } => Some(LinkedJsonFile {
                path: self.normalize_uri_path(package_id, &file.uri),
                file_type: *file_type,
                text: content.clone(),
                value: serde_json::from_str(content).unwrap_or_else(|error| {
                    panic!(
                        "failed to decode '{}' json output: {error}",
                        output_name.as_str()
                    )
                }),
            }),
            _ => panic!(
                "expected json output in optional '{}'",
                output_name.as_str()
            ),
        }
    }

    /// Return one exact output file from the given group.
    fn single_output_file<'a>(
        &self,
        output: &'a PackageOutput,
        output_name: TargetOutputName,
    ) -> &'a OutputFile {
        let files = output
            .outputs
            .get(&output_name)
            .unwrap_or_else(|| panic!("missing output group '{}'", output_name.as_str()));

        match files.as_slice() {
            [file] => file,
            _ => panic!(
                "expected exactly one output file in '{}', found {}",
                output_name.as_str(),
                files.len()
            ),
        }
    }

    /// Return all normalized text output files from the given groups.
    fn text_outputs_from_groups(
        &self,
        package_id: PackageId,
        output: &PackageOutput,
        output_names: &[TargetOutputName],
    ) -> IndexMap<String, LinkedTextFile> {
        let mut files = IndexMap::new();

        // collect files in the requested group order
        for output_name in output_names {
            let Some(group_files) = output.outputs.get(output_name) else {
                continue;
            };

            for file in group_files {
                let linked_file = match &file.content {
                    OutputContent::Text { code, file_type } => {
                        let path = self.normalize_uri_path(package_id, &file.uri);
                        LinkedTextFile {
                            path: path.clone(),
                            file_type: *file_type,
                            text: code.clone(),
                        }
                    }
                    _ => panic!("expected text output in '{}'", output_name.as_str()),
                };

                files.insert(linked_file.path.clone(), linked_file);
            }
        }

        files
    }

    /// Normalize one URI to a package-relative path when possible.
    fn normalize_uri_path(&self, package_id: PackageId, uri: &Uri) -> String {
        let path = uri
            .to_path()
            .unwrap_or_else(|| panic!("uri '{}' is not a path", uri));

        self.normalize_package_path(package_id, path)
    }

    /// Normalize one path to a package-relative path when possible.
    fn normalize_package_path(&self, package_id: PackageId, path: &Path) -> String {
        let package = self.program.package_descriptor(package_id);
        let package_dir = package
            .path
            .clone()
            .unwrap_or_else(|| self.program.root_directory().clone());
        let relative = path.strip_prefix(&package_dir).unwrap_or(path);

        relative.to_string_lossy().replace('\\', "/")
    }

    /// Assert one exact linked text file.
    pub(super) fn assert_linked_text_file(
        &self,
        actual: &LinkedTextFile,
        expected: &LinkedTextFile,
        noun: &str,
    ) {
        assert_eq!(actual.path, expected.path, "{noun} path mismatch");
        assert_eq!(
            actual.file_type, expected.file_type,
            "{noun} file type mismatch"
        );

        if actual.text != expected.text {
            print_diff(&expected.text, &actual.text, &DiffOptions::new());
            panic!("{noun} text mismatch");
        }
    }

    /// Assert one exact linked JSON file.
    pub(super) fn assert_linked_json_file<T>(
        &self,
        actual: &LinkedJsonFile<T>,
        expected: &LinkedJsonFile<T>,
        noun: &str,
    ) where
        T: PartialEq + Serialize + std::fmt::Debug,
    {
        assert_eq!(actual.path, expected.path, "{noun} path mismatch");
        assert_eq!(
            actual.file_type, expected.file_type,
            "{noun} file type mismatch"
        );

        if actual.text != expected.text {
            print_diff(&expected.text, &actual.text, &DiffOptions::new());
            panic!("{noun} text mismatch");
        }

        assert_eq!(actual.value, expected.value, "{noun} value mismatch");
    }

    /// Assert exact linked output groups.
    fn assert_output_groups(
        &self,
        actual: &IndexMap<TargetOutputName, Vec<String>>,
        expected: &IndexMap<TargetOutputName, Vec<String>>,
        noun: &str,
    ) {
        let actual = self.format_output_groups(actual);
        let expected = self.format_output_groups(expected);

        if actual != expected {
            print_diff(&expected, &actual, &DiffOptions::new());
            panic!("{noun} mismatch");
        }
    }

    /// Assert exact linked chunked module outputs.
    fn assert_linked_chunked_script_modules(
        &self,
        actual: &IndexMap<String, LinkedTextFile>,
        expected: &IndexMap<String, LinkedTextFile>,
    ) {
        let actual_paths = actual.keys().cloned().collect::<Vec<_>>();
        let expected_paths = expected.keys().cloned().collect::<Vec<_>>();

        assert_eq!(
            actual_paths, expected_paths,
            "linked chunked module output paths mismatch"
        );

        for (path, expected_file) in expected {
            let actual_file = actual
                .get(path)
                .unwrap_or_else(|| panic!("missing linked chunked module output '{path}'"));

            self.assert_linked_text_file(
                actual_file,
                expected_file,
                &format!("linked chunked module output '{path}'"),
            );
        }
    }

    /// Format linked output groups for stable diff output.
    fn format_output_groups(&self, groups: &IndexMap<TargetOutputName, Vec<String>>) -> String {
        groups
            .iter()
            .map(|(name, files)| format!("{}: {}", name.as_str(), files.join(", ")))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
